use alloy::consensus::{
    EthereumTxEnvelope, EthereumTypedTransaction, SignableTransaction, Signed, TxEip4844,
    TxEip4844Variant,
};
use alloy::hex;
use alloy::network::TransactionBuilder;
use alloy::primitives::{address, keccak256, Address, B256, U256};
use alloy::rpc::types::eth::TransactionRequest;
use alloy::rpc::types::TransactionReceipt;
use alloy_rlp::{Decodable, Encodable};
use anyhow::{anyhow, bail, Result};
use bigdecimal::BigDecimal;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::sync::{mpsc, oneshot};
use tokio::task::LocalSet;

use crate::client::TransactionConfig;
use crate::entity::{GolemBaseTransaction, Hash};
use crate::resilient_provider::ResilientProvider;
use crate::signers::TransactionSigner;
use crate::utils::eth_to_wei;

/// Helper function to display an Option value
fn display_option<T: std::fmt::Display>(opt: &Option<T>) -> String {
    opt.as_ref()
        .map(|n| n.to_string())
        .unwrap_or_else(|| "None".to_string())
}

/// Contains all three nonce values for an account
#[derive(Debug, Clone, derive_more::Display)]
#[display("Last tracked nonce: {}, next pending nonce: {next_pending_nonce}, current account nonce: {account_nonce}", display_option(&last_used_nonce))]
pub struct NonceInfo {
    /// Last nonce used by SDK code (saved during previous call to process_transaction)
    pub last_used_nonce: Option<u64>,
    /// Next nonce including pending transactions
    pub next_pending_nonce: u64,
    /// Current nonce value from blockchain (represents next nonce after last confirmed transaction)
    pub account_nonce: u64,
}

/// The address of the GolemBase storage processor contract.
/// All storage-related transactions are sent to this contract address.
pub const GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS: Address =
    address!("0x0000000000000000000000000000000060138453");

/// Response type for queued transactions.
/// Used internally for passing transaction results through channels.
type TransactionResponse = Result<TransactionReceipt>;

/// Channel for transaction response.
/// Allows awaiting the result of a queued transaction asynchronously.
pub struct TransactionChannel {
    response_rx: oneshot::Receiver<TransactionResponse>,
}

impl TransactionChannel {
    /// Awaits the transaction receipt from the queue worker.
    /// Returns the transaction receipt or an error if the channel is closed.
    pub async fn receipt(self) -> Result<TransactionReceipt> {
        self.response_rx
            .await
            .map_err(|e| anyhow!("Failed to get transaction response: {}", e))?
    }
}

/// Message type for the transaction queue.
/// Contains the transaction request and a channel to send the result back.
struct QueueMessage {
    request: TransactionRequest,
    response_tx: oneshot::Sender<TransactionResponse>,
}

/// Queue for managing transaction submissions.
/// Handles signing, sending, and awaiting receipts for transactions in a background worker.
struct TransactionQueue {
    sender: mpsc::Sender<QueueMessage>,
    signer: Arc<Box<dyn TransactionSigner>>,
    provider: ResilientProvider,
    tx_config: Arc<TransactionConfig>,
    /// Last nonce used by SDK code (saved during previous call to process_transaction)
    last_used_nonce: Mutex<Option<u64>>,
}

/// Event signature for extending BTL (block time to live) of an entity.
/// Used to identify `GolemBaseStorageEntityBTLExtended` events in logs.
pub fn golem_base_storage_entity_btl_extended() -> B256 {
    keccak256(b"GolemBaseStorageEntityBTLExtended(uint256,uint256)")
}

impl TransactionQueue {
    /// Creates a new transaction queue and spawns a worker task to process transactions.
    /// The worker signs, sends, and tracks receipts for all queued transactions.
    fn new(
        provider: ResilientProvider,
        signer: Arc<Box<dyn TransactionSigner>>,
        tx_config: Arc<TransactionConfig>,
    ) -> Arc<Self> {
        let (tx, rx) = mpsc::channel(32);
        let queue = Arc::new(Self {
            sender: tx,
            signer,
            provider,
            tx_config,
            last_used_nonce: Mutex::new(None),
        });
        Self::spawn_worker(rx, queue.clone());
        queue
    }

    /// Signs a transaction request using the account's signer.
    /// Returns the signed transaction ready for encoding and submission.
    async fn sign_transaction(
        &self,
        tx: TransactionRequest,
    ) -> anyhow::Result<Signed<EthereumTypedTransaction<TxEip4844Variant>>> {
        let tx = tx.build_unsigned()?;
        let bytes = tx.encoded_for_signing();

        let signature = self.signer.sign(&bytes).await?;
        Ok(tx.into_signed(signature))
    }

    /// Encodes a signed transaction to RLP bytes for network submission.
    /// Also logs the transaction hash and attempts to decode and recover the signer for debugging.
    fn encode_transaction(
        &self,
        signed: &Signed<EthereumTypedTransaction<TxEip4844Variant>>,
    ) -> anyhow::Result<Vec<u8>> {
        let mut encoded = Vec::new();
        signed.eip2718_encode(&mut encoded);

        log::trace!(
            "RLP encoded transaction (hash: 0x{:x}): 0x{}",
            signed.hash(),
            hex::encode(&encoded)
        );

        // Decode the transaction for debugging purposes.
        let decoded_tx = EthereumTxEnvelope::<TxEip4844>::decode(&mut &encoded[..])
            .map_err(|e| anyhow!("Failed to decode transaction: {e}"))?;
        log::debug!("Decoded transaction: {:#?}", decoded_tx);

        let signer = decoded_tx
            .recover_signer()
            .map_err(|e| anyhow!("Failed to recover signer: {e}"))?;
        log::debug!("Recovered signer: {:#?}", signer);

        Ok(encoded)
    }

    /// Gets a transaction receipt with retries for "transaction indexing is in progress" errors.
    /// Waits until the transaction is indexed and the receipt is available, or returns None if timeout.
    async fn get_receipt_with_retry(
        &self,
        tx_hash: Hash,
    ) -> anyhow::Result<Option<TransactionReceipt>> {
        let timeout = self.tx_config.transaction_receipt_timeout.clone();
        get_receipt(
            &self.provider,
            tx_hash,
            Some(timeout),
            self.tx_config.required_confirmations,
        )
        .await
    }

    /// Returns a new TransactionRequest with bumped tip and fee cap by a percentage for replacement transactions.
    fn bump_fees(&self, request: &TransactionRequest, attempt: u32) -> TransactionRequest {
        let bump_percent = self.tx_config.price_bump_percent * attempt as u128;
        let tip = request
            .max_priority_fee_per_gas
            .unwrap_or(self.tx_config.max_priority_fee_per_gas);
        let fee_cap = request
            .max_fee_per_gas
            .unwrap_or(self.tx_config.max_fee_per_gas);
        let bumped_tip = tip + (tip * bump_percent).div_ceil(100);
        let bumped_fee_cap = fee_cap + (fee_cap * bump_percent).div_ceil(100);
        request
            .clone()
            .with_max_priority_fee_per_gas(bumped_tip)
            .with_max_fee_per_gas(bumped_fee_cap)
    }

    /// Processes a single transaction:
    /// - Gets the current nonce for the sender.
    /// - Signs and encodes the transaction.
    /// - Sends the transaction and waits for it to be mined.
    /// - Returns the transaction receipt.
    async fn process_transaction(&self, request: TransactionRequest) -> TransactionResponse {
        // Get the current nonce for the sender address.
        let from = request
            .from
            .ok_or_else(|| anyhow!("Transaction request missing 'from' address"))?;

        // We have 2 sources of nonces: our last used nonce and RPC.
        // RPC returns nonce of last pending transaction and nonce associated with the account.
        // Since we have no guarantee of sending requests to the same RPC, we can't trust fully
        // that it have full knowledge the returned nonces. At the same time tools outside of our
        // control can send transactions as well.
        let nonce_info = self.get_nonces(from).await?;
        let nonce = match nonce_info.last_used_nonce {
            Some(last_used) => std::cmp::max(nonce_info.next_pending_nonce, last_used + 1),
            None => nonce_info.next_pending_nonce,
        };

        log::info!("Nonce info: {nonce_info}");

        let pending = nonce_info.next_pending_nonce as i64 - (nonce_info.account_nonce as i64 + 1);
        if pending > 0 {
            log::debug!("Still processing {pending} pending transactions");
        }

        if let Some(last_used) = nonce_info.last_used_nonce {
            if (last_used + 1) != nonce_info.next_pending_nonce {
                log::warn!("Last used nonce is not equal to next pending nonce. Probably transaction was sent externally.");
            }
        }

        // Update the request with the next pending nonce.
        let request = request.with_nonce(nonce);

        let max_retries = self.tx_config.max_retries;
        let mut attempt: u32 = 0;

        loop {
            let _request = match attempt > 0 {
                false => request.clone(),
                // Bump both tip and fee cap by configured percent per attempt.
                // Otherwise we will either get rejected due to sending exacly the same transaction
                // (`already known` error) or we will get `replacement transaction underpriced` error
                // if we don't bump gas prices enough.
                true => self.bump_fees(&request, attempt),
            };

            // Sign and encode the transaction.
            let signed = self.sign_transaction(_request).await?;
            let encoded = self.encode_transaction(&signed)?;

            // Send the transaction and register it for tracking.
            let pending = self
                .provider
                .send_raw_transaction(&encoded)
                .await
                .map_err(|e| anyhow!("Failed to send transaction: {e}"))?;

            let tx_hash = *pending.tx_hash();
            attempt += 1;

            log::debug!("Transaction attempt {attempt} sent with hash: {tx_hash}");

            if let Some(receipt) = self.get_receipt_with_retry(tx_hash).await? {
                log::info!("Transaction succeeded on attempt {attempt} with hash: {tx_hash}");

                self.set_last_used_nonce(nonce);
                return Ok(receipt);
            }

            if attempt >= max_retries {
                return Err(anyhow!(
                    "Transaction failed after {max_retries} attempts, last hash: {tx_hash}"
                ));
            }

            log::warn!("Transaction attempt {attempt} timed out (hash: {tx_hash}), retrying...",);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Spawns a worker task to process queued transactions in the background.
    /// The worker receives transaction requests, processes them, and sends back receipts.
    fn spawn_worker(mut rx: mpsc::Receiver<QueueMessage>, queue: Arc<Self>) {
        let runtime = Builder::new_current_thread().enable_all().build().unwrap();

        // We have 2 options to spawn signing worker task: using `spawn` or `spawn_local`.
        // Using `spawn_local` can panic when called outside of LocalSet. That means that
        // we force library consumer to use actix runtime or to manually create LocalSet.
        // On the other hand using `spawn` will prevent consumer from using `spawn_local` in
        // signing function.
        // Spawning thread here might be overkill, but it's the only way to avoid affecting users.
        std::thread::spawn(move || {
            let local = LocalSet::new();

            local.spawn_local(async move {
                while let Some(msg) = rx.recv().await {
                    let QueueMessage {
                        request,
                        response_tx,
                    } = msg;
                    let result = queue.process_transaction(request).await;
                    let _ = response_tx.send(result);
                }
            });

            runtime.block_on(local);
        });
    }

    /// Gets all three nonce values for this account.
    /// Returns a NonceInfo struct containing last_used_nonce, next_pending_nonce, and current_blockchain_nonce.
    async fn get_nonces(&self, address: Address) -> Result<NonceInfo> {
        // Get last used nonce from stored value
        let last_used_nonce = *self.last_used_nonce.lock().unwrap();

        // Get current blockchain nonce from get_nonce. This function includes only
        // confirmed transactions.
        //let account_nonce = self.provider.get_nonce(address).await?;
        let account_nonce = 0;

        // Get next pending nonce from get_transaction_count. This function includes
        // pending transactions as well.
        let next_pending_nonce = self.provider.get_transaction_count(address).await?;

        Ok(NonceInfo {
            last_used_nonce,
            next_pending_nonce,
            account_nonce,
        })
    }

    /// Sets the last used nonce for this account.
    fn set_last_used_nonce(&self, nonce: u64) {
        let mut last_used = self.last_used_nonce.lock().unwrap();
        *last_used = Some(nonce);
    }

    /// Queues a transaction for processing and returns a channel to await the result.
    /// The transaction will be signed, sent, and the receipt returned asynchronously.
    async fn queue_transaction(&self, request: TransactionRequest) -> Result<TransactionChannel> {
        let (response_tx, response_rx) = oneshot::channel();
        let msg = QueueMessage {
            request,
            response_tx,
        };
        self.sender
            .send(msg)
            .await
            .map_err(|e| anyhow!("Failed to queue transaction: {}", e))?;
        Ok(TransactionChannel { response_rx })
    }
}

/// An account with its signer.
/// Provides methods for sending transactions, funding, and interacting with GolemBase storage.
#[derive(Clone)]
pub struct Account {
    /// The account's signer for signing transactions.
    pub signer: Arc<Box<dyn TransactionSigner>>,
    /// The provider for making RPC calls.
    pub provider: ResilientProvider,
    /// The chain ID of the connected network.
    pub chain_id: u64,
    /// Transaction queue for managing transaction submissions.
    transaction_queue: Arc<TransactionQueue>,
    /// Transaction configuration for storage operations.
    tx_config: Arc<TransactionConfig>,
}

impl Account {
    /// Creates a new account with the given signer, provider, chain ID, and transaction config.
    /// Initializes a transaction queue for managing transaction submissions.
    pub fn new(
        signer: Box<dyn TransactionSigner>,
        provider: ResilientProvider,
        chain_id: u64,
        tx_config: Arc<TransactionConfig>,
    ) -> Self {
        let signer = Arc::new(signer);
        let transaction_queue =
            TransactionQueue::new(provider.clone(), signer.clone(), tx_config.clone());
        Self {
            signer,
            provider,
            chain_id,
            transaction_queue,
            tx_config,
        }
    }

    /// Returns the Ethereum address of this account.
    pub fn address(&self) -> Address {
        self.signer.address()
    }

    /// Sends a transaction with common fields filled in (from, chain_id).
    /// Queues the transaction for signing and submission, and awaits the receipt.
    pub async fn send_transaction(&self, mut tx: TransactionRequest) -> Result<TransactionReceipt> {
        // Fill in common fields
        tx = tx.with_from(self.address()).with_chain_id(self.chain_id);

        // Queue the raw transaction (unsigned)
        let channel = self.transaction_queue.queue_transaction(tx).await?;
        channel.receipt().await
    }

    /// Creates and sends a storage transaction to the GolemBase contract.
    /// Encodes the transaction payload and submits it to the storage processor contract.
    pub async fn send_db_transaction(
        &self,
        tx: GolemBaseTransaction,
    ) -> Result<TransactionReceipt> {
        let mut data = Vec::new();
        tx.encode(&mut data);

        let tx = TransactionRequest::default()
            .with_to(GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS)
            .with_gas_limit(self.tx_config.gas_limit)
            .with_max_priority_fee_per_gas(self.tx_config.max_priority_fee_per_gas)
            .with_max_fee_per_gas(self.tx_config.max_fee_per_gas)
            .with_input(data.to_vec());

        self.send_transaction(tx).await
    }

    /// Transfers ETH from this account to another address.
    /// Returns the transaction receipt after the transfer is mined.
    pub async fn transfer(&self, to: Address, value: BigDecimal) -> Result<TransactionReceipt> {
        let tx = TransactionRequest::default()
            .with_to(to)
            .with_value(eth_to_wei(value)?)
            .with_gas_limit(21_000)
            .with_max_priority_fee_per_gas(1_000_000)
            .with_max_fee_per_gas(20_000_000);

        self.send_transaction(tx).await
    }

    /// Funds an account by sending ETH from a node-managed account.
    /// This is typically used in development mode for test funding.
    pub async fn fund_account(&self, value: BigDecimal) -> anyhow::Result<TransactionReceipt> {
        let accounts = self.provider.get_accounts().await?;
        let funder = accounts[0];

        let nonce = self.provider.get_transaction_count(funder).await?;

        let tx = TransactionRequest::default()
            .with_to(self.address())
            .with_from(funder)
            .with_value(eth_to_wei(value)?)
            .with_nonce(nonce)
            .with_chain_id(self.chain_id)
            .with_gas_limit(21_000)
            .with_max_priority_fee_per_gas(1_000_000_000)
            .with_max_fee_per_gas(20_000_000_000);

        let pending = self
            .provider
            .send_transaction(tx)
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;
        self.transaction_queue
            .get_receipt_with_retry(*pending.tx_hash())
            .await?
            .ok_or_else(|| anyhow!("Transaction receipt not found for funding transaction"))
    }

    /// Gets the account's ETH balance from the provider.
    /// Returns the balance as a U256 value.
    pub async fn get_balance(&self) -> anyhow::Result<U256> {
        Ok(self.provider.get_balance(self.address()).await?)
    }
}

/// Gets a transaction receipt with retries for "transaction indexing is in progress" errors.
/// Waits until the transaction is indexed and the receipt is available, or returns an error.
/// If a timeout is provided and no receipt is received within that time, returns None.
pub async fn get_receipt(
    provider: &ResilientProvider,
    tx_hash: Hash,
    timeout_duration: Option<Duration>,
    confirmations: u64,
) -> anyhow::Result<Option<TransactionReceipt>> {
    let start_time = std::time::Instant::now();

    if let None = provider.get_transaction_by_hash(tx_hash).await? {
        log::debug!("Transaction {tx_hash} wasn't send to seqencer properly. Caller should retry.");
        return Ok(None);
    }

    loop {
        // Check if we've exceeded the timeout
        if let Some(duration) = timeout_duration {
            if start_time.elapsed() >= duration {
                return Ok(None);
            }
        }

        // Must be checked before getting the receipt, to avoid race conditions.
        let tx = provider.get_transaction_by_hash(tx_hash).await?;

        match provider.get_transaction_receipt(tx_hash).await {
            Ok(opt_receipt) => match opt_receipt {
                Some(receipt) => {
                    log::info!(
                        "Transaction {tx_hash} was included in a block {:?} ({:?}). Waiting for {confirmations} confirmations.",
                        receipt.block_number,
                        receipt.block_hash.map(|b| hex::encode(b))
                    );
                    provider
                        .watch_for_confirmation(tx_hash, confirmations)
                        .await?;
                    return Ok(Some(receipt));
                }
                _ => {
                    log::trace!("Getting receipt returned None for transaction: {tx_hash}");

                    if let Some(tx) = tx {
                        if tx.block_hash.is_some() {
                            log::debug!(
                            "Transaction {tx_hash} was already included in a block {:?} ({:?}), but receipt is not available.",
                            tx.block_number,
                            tx.block_hash.map(|b| hex::encode(b))
                        );
                            bail!("Transaction {tx_hash} was already included in a block, but we are not able to get the receipt.");
                        }
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    continue;
                }
            },
            Err(e)
                if e.to_string()
                    .contains("transaction indexing is in progress") =>
            {
                log::debug!("Ignoring `indexing is in progress` error for transaction: {tx_hash}");
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }

            Err(e) => {
                return Err(anyhow!("Failed to get transaction receipt: {}", e));
            }
        }
    }
}
