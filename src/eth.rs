use crate::GolemBaseClient;
use crate::entity::{
    Create, DeleteResult, EntityResult, Extend, ExtendResult, GolemBaseTransaction, Update,
};
use crate::entity::{Hash, TransactionResult};

use alloy::network::TransactionBuilder;
use alloy::primitives::{Address, TxKind, address};
use alloy::providers::Provider;
use alloy::rpc::types::{TransactionReceipt, TransactionRequest};
use displaydoc::Display;
use thiserror::Error;

alloy::sol! {
    contract GolemBaseABI {
        event GolemBaseStorageEntityCreated(
            uint256 indexed entityKey,
            uint256 expirationBlock
        );

        event GolemBaseStorageEntityUpdated(
            uint256 indexed entityKey,
            uint256 expirationBlock
        );

        event GolemBaseStorageEntityDeleted(
            uint256 indexed entityKey
        );

        event GolemBaseStorageEntityBTLExtended(
            uint256 indexed entityKey,
            uint256 oldExpirationBlock,
            uint256 newExpirationBlock
        );
    }
}

/// Represents errors that can occur in the GolemBase ETH client.
/// Used for wrapping transaction, receipt, and log decoding errors.
#[derive(Debug, Display, Error)]
pub enum Error {
    /// Failed to send transaction: {0}
    TransactionSendError(String),
    /// Failed to get transaction receipt: {0}
    TransactionReceiptError(String),
    /// Unexpected log data: {0}
    UnexpectedLogDataError(String),
}

/// The Ethereum address of the GolemBase storage contract.
/// All entity-related transactions are sent to this address.
pub const STORAGE_ADDRESS: Address = address!("0x0000000000000000000000000000000060138453");

impl GolemBaseClient {
    pub async fn send_transaction(
        &self,
        tx: GolemBaseTransaction,
    ) -> Result<TransactionResult, Error> {
        let receipt = self.create_raw_transaction(tx).await?;
        receipt.try_into()
    }

    /// Creates one or more new entities in GolemBase and returns their results.
    /// Sends a transaction to the storage contract and parses the resulting logs.
    pub async fn create_entities(&self, creates: Vec<Create>) -> Result<Vec<EntityResult>, Error> {
        let result = self
            .send_transaction(GolemBaseTransaction::builder().creates(creates).build())
            .await;

        result.and_then(|res| match res {
            TransactionResult {
                creates,
                updates,
                deletes,
                extensions,
            } if updates.is_empty() && deletes.is_empty() && extensions.is_empty() => Ok(creates),
            _ => Err(Error::UnexpectedLogDataError(
                "Unexpected content in tx logs, this should never happen!".to_string(),
            )),
        })
    }

    /// Updates one or more entities in GolemBase and returns their results.
    /// Sends a transaction to the storage contract and parses the resulting logs.
    pub async fn update_entities(&self, updates: Vec<Update>) -> Result<Vec<EntityResult>, Error> {
        let result = self
            .send_transaction(GolemBaseTransaction::builder().updates(updates).build())
            .await;

        result.and_then(|res| match res {
            TransactionResult {
                creates,
                updates,
                deletes,
                extensions,
            } if creates.is_empty() && deletes.is_empty() && extensions.is_empty() => Ok(updates),
            _ => Err(Error::UnexpectedLogDataError(
                "Unexpected content in tx logs, this should never happen!".to_string(),
            )),
        })
    }

    /// Deletes one or more entities in GolemBase and returns their results.
    /// Sends a transaction to the storage contract and parses the resulting logs.
    pub async fn delete_entities(&self, deletes: Vec<Hash>) -> Result<Vec<DeleteResult>, Error> {
        let result = self
            .send_transaction(GolemBaseTransaction::builder().deletes(deletes).build())
            .await;

        result.and_then(|res| match res {
            TransactionResult {
                creates,
                updates,
                deletes,
                extensions,
            } if creates.is_empty() && updates.is_empty() && extensions.is_empty() => Ok(deletes),
            _ => Err(Error::UnexpectedLogDataError(
                "Unexpected content in tx logs, this should never happen!".to_string(),
            )),
        })
    }

    /// Extends the BTL (block time to live) of one or more entities and returns their results.
    /// Sends a transaction to the storage contract and parses the resulting logs for old and new expiration blocks.
    pub async fn extend_entities(
        &self,
        extensions: Vec<Extend>,
    ) -> Result<Vec<ExtendResult>, Error> {
        let result = self
            .send_transaction(
                GolemBaseTransaction::builder()
                    .extensions(extensions)
                    .build(),
            )
            .await;

        result.and_then(|res| match res {
            TransactionResult {
                creates,
                updates,
                deletes,
                extensions,
            } if creates.is_empty() && updates.is_empty() && deletes.is_empty() => Ok(extensions),
            _ => Err(Error::UnexpectedLogDataError(
                "Unexpected content in tx logs, this should never happen!".to_string(),
            )),
        })
    }

    /// NOTE: Nonce management is tricky!
    /// - This implementation always tries to fetch the latest on-chain nonce before sending a transaction,
    ///   and only falls back to the locally cached base_nonce if the sync fails.
    /// - Only the number of in-flight transactions is tracked locally.
    /// - For robust production use, consider also handling stuck transactions (e.g., gas bumping/EIP-1559).
    async fn next_nonce(&self) -> u64 {
        // This is sadly needed because `self.provider.get_transaction_count(self.wallet.address()).pending()`
        // doesn't give the right number...
        //
        //      Error: server returned an error response: error code -32000: replacement transaction underpriced
        let mut nm = self.nonce_manager.lock().await;
        let wallet_address = self.wallet.address();
        match self.provider.get_transaction_count(wallet_address).await {
            Ok(on_chain_nonce) => {
                nm.base_nonce = on_chain_nonce;
            }
            Err(e) => {
                tracing::warn!("Failed to fetch on-chain nonce: {e}");
            }
        }
        nm.next_nonce().await
    }

    /// Creates and sends a raw transaction to the GolemBase storage contract.
    /// Encodes the transaction payload and sends it to the contract address.
    pub async fn create_raw_transaction(
        &self,
        payload: GolemBaseTransaction,
    ) -> Result<TransactionReceipt, Error> {
        tracing::debug!("payload: {payload:?}");
        let encoded = payload.encoded();
        tracing::debug!("buffer: {encoded:?}");

        let nonce = self.next_nonce().await;

        let mut tx = TransactionRequest {
            to: Some(TxKind::Call(STORAGE_ADDRESS)),
            input: encoded.into(),
            chain_id: Some(
                self.provider
                    .get_chain_id()
                    .await
                    .map_err(|e| Error::TransactionSendError(e.to_string()))?,
            ),
            nonce: Some(nonce),
            ..Default::default()
        };
        tracing::debug!("transaction: {tx:?}");

        let gas_limit = if let Some(gas_limit) = payload.gas_limit {
            gas_limit
        } else {
            self.provider
                .estimate_gas(tx.clone())
                .await
                .map_err(|e| Error::TransactionSendError(format!("Failed to estimate gas: {e}")))?
        };
        tx = tx.with_gas_limit(gas_limit);

        if let Some(max_fee_per_gas) = payload.max_fee_per_gas {
            tx = tx.with_max_fee_per_gas(max_fee_per_gas);
        }

        if let Some(max_priority_fee_per_gas) = payload.max_priority_fee_per_gas {
            tx = tx.with_max_priority_fee_per_gas(max_priority_fee_per_gas);
        }

        let pending_tx = self
            .provider
            .send_transaction(tx.clone())
            .await
            .map_err(|e| Error::TransactionSendError(e.to_string()))?;
        tracing::debug!("pending transaction: {pending_tx:?}");
        let receipt = pending_tx
            .get_receipt()
            .await
            .map_err(|e| Error::TransactionReceiptError(e.to_string()))?;
        tracing::debug!("receipt: {receipt:?}");
        {
            let mut nm = self.nonce_manager.lock().await;
            nm.complete().await;
        }

        if !receipt.status() {
            self.provider.call(tx).await.map_err(|e| {
                Error::TransactionReceiptError(format!("Error during tx execution: {e}"))
            })?;
        }

        Ok(receipt)
    }
}
