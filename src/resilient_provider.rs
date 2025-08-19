use std::time::Duration;

use alloy::eips::BlockNumberOrTag;
use alloy::network::{Ethereum, Network};
use alloy::primitives::{Address, B256, U256};
use alloy::providers::{
    DynProvider, PendingTransactionBuilder, PendingTransactionConfig, Provider,
};
use alloy::rpc::types::SyncStatus;
use anyhow::{anyhow, Result};

/// Configuration for the resilient provider retry behavior.
#[derive(Clone, Debug)]
pub struct ResilientProviderConfig {
    /// Maximum number of retry attempts for "error sending request" errors.
    pub max_retries: u32,
    /// Delay between retry attempts in milliseconds.
    pub retry_delay_ms: u64,
    /// Timeout in seconds for "no backend is currently healthy" errors.
    pub backend_health_timeout_secs: u64,
}

impl Default for ResilientProviderConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 100,
            backend_health_timeout_secs: 30,
        }
    }
}

/// A wrapper around DynProvider that handles "error sending request" errors by retrying.
/// This provides resilience against temporary network issues and RPC load balancing switches.
#[derive(Clone)]
pub struct ResilientProvider<N = Ethereum> {
    provider: DynProvider<N>,
    config: ResilientProviderConfig,
}

impl<N> ResilientProvider<N>
where
    N: Network,
{
    /// Creates a new ResilientProvider with the given provider and configuration.
    pub fn new(provider: DynProvider<N>, config: ResilientProviderConfig) -> Self {
        Self { provider, config }
    }

    /// Creates a new ResilientProvider with default configuration.
    pub fn new_with_default_config(provider: DynProvider<N>) -> Self {
        Self::new(provider, ResilientProviderConfig::default())
    }

    /// Gets the underlying provider.
    pub fn inner(&self) -> &DynProvider<N> {
        &self.provider
    }

    /// Gets the retry configuration.
    pub fn config(&self) -> &ResilientProviderConfig {
        &self.config
    }

    /// Generic retry function that handles "error sending request" errors and "no backend is currently healthy" errors.
    async fn retry<T, F, Fut, E>(&self, operation_name: &str, operation: F) -> anyhow::Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut attempts = 0;
        let start_time = std::time::Instant::now();

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                // This error can be result of load balancer switching.
                // It should be enough to retry a few times for the call to work.
                Err(e) if e.to_string().contains("error sending request") => {
                    attempts += 1;
                    if attempts > self.config.max_retries {
                        return Err(anyhow!(
                            "{operation_name} failed after {} attempts with 'error sending request': {e}",
                            self.config.max_retries,
                        ));
                    }
                    log::debug!(
                            "{operation_name} failed with 'error sending request' (attempt {attempts}/{}). Retrying...",
                            self.config.max_retries
                        );
                    continue;
                }
                // Load balancer will be switching backends until it will find a healthy one.
                // We need to retry and wait for some period of time.
                Err(e)
                    if e.to_string()
                        .contains("no backend is currently healthy to serve traffic") =>
                {
                    let elapsed = start_time.elapsed();
                    let timeout_duration =
                        Duration::from_secs(self.config.backend_health_timeout_secs);

                    if elapsed >= timeout_duration {
                        return Err(anyhow!(
                            "{operation_name} failed after {} seconds with 'no backend is currently healthy to serve traffic': {e}",
                            self.config.backend_health_timeout_secs,
                        ));
                    }

                    log::debug!(
                        "{operation_name} failed with 'no backend is currently healthy to serve traffic' (elapsed: {:.1}s/{:.1}s). Retrying...",
                        elapsed.as_secs_f64(),
                        timeout_duration.as_secs_f64()
                    );
                    tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                    continue;
                }
                Err(e) => return Err(anyhow!("{e}")),
            }
        }
    }

    /// Gets the transaction count (nonce) for an address with retry logic.
    pub async fn get_transaction_count(&self, address: Address) -> Result<u64> {
        self.retry("get_transaction_count", || async {
            self.provider.get_transaction_count(address).await
        })
        .await
    }

    /// Gets account nonce using the RPC get_proof method with retry logic.
    /// This returns the confirmed nonce (excluding pending transactions).
    pub async fn get_nonce(&self, address: Address) -> Result<u64> {
        let address_clone = address;
        self.retry("get_proof", move || async move {
            match self.provider.get_proof(address_clone, vec![]).await {
                Ok(proof_response) => Ok(proof_response.nonce),
                Err(e) => Err(anyhow!("Failed to get proof: {}", e)),
            }
        })
        .await
    }

    /// Gets the balance for an address with retry logic.
    pub async fn get_balance(&self, address: Address) -> Result<U256> {
        self.retry("get_balance", || async {
            self.provider.get_balance(address).await
        })
        .await
    }

    /// Gets accounts with retry logic.
    pub async fn get_accounts(&self) -> Result<Vec<Address>> {
        self.retry("get_accounts", || self.provider.get_accounts())
            .await
    }

    /// Sends a transaction with retry logic.
    pub async fn send_transaction(
        &self,
        tx: N::TransactionRequest,
    ) -> Result<PendingTransactionBuilder<N>> {
        let tx_clone = tx.clone();
        self.retry("send_transaction", || {
            self.provider.send_transaction(tx_clone.clone())
        })
        .await
    }

    /// Sends a raw transaction with retry logic.
    pub async fn send_raw_transaction(&self, data: &[u8]) -> Result<PendingTransactionBuilder<N>> {
        let data = data.to_vec();
        self.retry("send_raw_transaction", || {
            self.provider.send_raw_transaction(&data)
        })
        .await
    }

    /// Gets a transaction by hash with retry logic.
    pub async fn get_transaction_by_hash(
        &self,
        hash: alloy::primitives::B256,
    ) -> Result<Option<N::TransactionResponse>> {
        self.retry("get_transaction_by_hash", || {
            self.provider.get_transaction_by_hash(hash)
        })
        .await
    }

    /// Gets a transaction receipt with retry logic.
    pub async fn get_transaction_receipt(
        &self,
        hash: alloy::primitives::B256,
    ) -> Result<Option<N::ReceiptResponse>> {
        self.retry("get_transaction_receipt", || {
            self.provider.get_transaction_receipt(hash)
        })
        .await
    }

    /// Gets the chain ID with retry logic.
    pub async fn get_chain_id(&self) -> Result<u64> {
        self.retry("get_chain_id", || self.provider.get_chain_id())
            .await
    }

    /// Gets the syncing status with retry logic.
    pub async fn syncing(&self) -> Result<SyncStatus> {
        self.retry("syncing", || self.provider.syncing()).await
    }

    /// Gets a block by number with retry logic.
    pub async fn get_block_by_number(
        &self,
        block: BlockNumberOrTag,
    ) -> Result<Option<N::BlockResponse>> {
        self.retry("get_block_by_number", || async {
            self.provider.get_block_by_number(block).await
        })
        .await
    }

    /// Gets a transaction by sender address and nonce with retry logic.
    pub async fn get_transaction_by_sender_nonce(
        &self,
        sender: Address,
        nonce: u64,
    ) -> Result<Option<N::TransactionResponse>> {
        self.retry("get_transaction_by_sender_nonce", || async {
            self.provider
                .get_transaction_by_sender_nonce(sender, nonce)
                .await
        })
        .await
    }

    /// Watches a pending transaction and waits for the specified number of confirmations with retry logic.
    pub async fn watch_for_confirmation(&self, tx_hash: B256, confirmations: u64) -> Result<()> {
        if confirmations == 0 {
            return Ok(());
        }

        let config =
            PendingTransactionConfig::new(tx_hash).with_required_confirmations(confirmations);

        let pending_tx = self
            .retry("watch_pending_transaction", || async {
                self.provider
                    .watch_pending_transaction(config.clone())
                    .await
            })
            .await?;

        // Wait for the transaction to be confirmed with the specified number of confirmations
        pending_tx.await?;
        Ok(())
    }
}

impl<N> From<DynProvider<N>> for ResilientProvider<N>
where
    N: Network,
{
    fn from(provider: DynProvider<N>) -> Self {
        Self::new_with_default_config(provider)
    }
}

impl<N> From<ResilientProvider<N>> for DynProvider<N>
where
    N: Network,
{
    fn from(resilient_provider: ResilientProvider<N>) -> Self {
        resilient_provider.provider
    }
}
