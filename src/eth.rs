use crate::GolemBaseClient;
use crate::entity::{
    Create, DeleteResult, EntityResult, Extend, ExtendResult, GolemBaseTransaction, Update,
};
use crate::entity::{Hash, TransactionResult};

use alloy;
use alloy::network::TransactionBuilder;
use alloy::primitives::{Address, TxKind, address};
use alloy::providers::Provider;
use alloy::rpc::types::{TransactionReceipt, TransactionRequest};
use alloy_rlp::Encodable;
use alloy_sol_types::{SolEventInterface, sol};
use displaydoc::Display;
use thiserror::Error;

sol! {
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
    /// Failed to decode expiration block: {0}
    ExpirationBlockDecodeError(String),
    /// Unexpected log data format
    UnexpectedLogDataError,
}

/// The Ethereum address of the GolemBase storage contract.
/// All entity-related transactions are sent to this address.
pub const STORAGE_ADDRESS: Address = address!("0x0000000000000000000000000000000060138453");

impl GolemBaseClient {
    pub async fn send_transaction(
        &self,
        creates: Vec<Create>,
        updates: Vec<Update>,
        deletes: Vec<Hash>,
        extensions: Vec<Extend>,
    ) -> Result<TransactionResult, Error> {
        let receipt = self
            .create_raw_transaction(GolemBaseTransaction {
                creates,
                updates,
                deletes,
                extensions,
            })
            .await?;

        let mut txres = TransactionResult::default();
        receipt.logs().iter().cloned().try_for_each(|log| {
            let log: alloy::primitives::Log = log.into();
            let parsed = GolemBaseABI::GolemBaseABIEvents::decode_log(&log).map_err(|e| {
                Error::TransactionReceiptError(format!("Error decoding event log: {}", e))
            })?;
            match parsed.data {
                GolemBaseABI::GolemBaseABIEvents::GolemBaseStorageEntityCreated(data) => {
                    txres.creates.push(EntityResult {
                        entity_key: data.entityKey.into(),
                        expiration_block: data.expirationBlock.try_into().unwrap_or_default(),
                    });
                    Ok(())
                }
                GolemBaseABI::GolemBaseABIEvents::GolemBaseStorageEntityUpdated(data) => {
                    txres.updates.push(EntityResult {
                        entity_key: data.entityKey.into(),
                        expiration_block: data.expirationBlock.try_into().unwrap_or_default(),
                    });
                    Ok(())
                }
                GolemBaseABI::GolemBaseABIEvents::GolemBaseStorageEntityDeleted(data) => {
                    txres.deletes.push(DeleteResult {
                        entity_key: data.entityKey.into(),
                    });
                    Ok(())
                }
                GolemBaseABI::GolemBaseABIEvents::GolemBaseStorageEntityBTLExtended(data) => {
                    txres.extensions.push(ExtendResult {
                        entity_key: data.entityKey.into(),
                        old_expiration_block: data
                            .oldExpirationBlock
                            .try_into()
                            .unwrap_or_default(),
                        new_expiration_block: data
                            .newExpirationBlock
                            .try_into()
                            .unwrap_or_default(),
                    });
                    Ok(())
                }
            }
        })?;

        Ok(txres)
    }

    /// Creates one or more new entities in GolemBase and returns their results.
    /// Sends a transaction to the storage contract and parses the resulting logs.
    pub async fn create_entities(&self, creates: Vec<Create>) -> Result<Vec<EntityResult>, Error> {
        let result = self.send_transaction(creates, vec![], vec![], vec![]).await;
        result.and_then(|res| match res {
            TransactionResult {
                creates,
                updates,
                deletes,
                extensions,
            } if updates.is_empty() && deletes.is_empty() && extensions.is_empty() => Ok(creates),
            _ => Err(Error::TransactionReceiptError(
                "Unexpected content in tx logs, this should never happen!".to_string(),
            )),
        })
    }

    /// Updates one or more entities in GolemBase and returns their results.
    /// Sends a transaction to the storage contract and parses the resulting logs.
    pub async fn update_entities(&self, updates: Vec<Update>) -> Result<Vec<EntityResult>, Error> {
        let result = self.send_transaction(vec![], updates, vec![], vec![]).await;
        result.and_then(|res| match res {
            TransactionResult {
                creates,
                updates,
                deletes,
                extensions,
            } if creates.is_empty() && deletes.is_empty() && extensions.is_empty() => Ok(updates),
            _ => Err(Error::TransactionReceiptError(
                "Unexpected content in tx logs, this should never happen!".to_string(),
            )),
        })
    }

    /// Deletes one or more entities in GolemBase and returns their results.
    /// Sends a transaction to the storage contract and parses the resulting logs.
    pub async fn delete_entities(&self, deletes: Vec<Hash>) -> Result<Vec<DeleteResult>, Error> {
        let result = self.send_transaction(vec![], vec![], deletes, vec![]).await;
        result.and_then(|res| match res {
            TransactionResult {
                creates,
                updates,
                deletes,
                extensions,
            } if creates.is_empty() && updates.is_empty() && extensions.is_empty() => Ok(deletes),
            _ => Err(Error::TransactionReceiptError(
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
            .send_transaction(vec![], vec![], vec![], extensions)
            .await;
        result.and_then(|res| match res {
            TransactionResult {
                creates,
                updates,
                deletes,
                extensions,
            } if creates.is_empty() && updates.is_empty() && deletes.is_empty() => Ok(extensions),
            _ => Err(Error::TransactionReceiptError(
                "Unexpected content in tx logs, this should never happen!".to_string(),
            )),
        })
    }

    /// Creates and sends a raw transaction to the GolemBase storage contract.
    /// Encodes the transaction payload and sends it to the contract address.
    ///
    /// NOTE: Nonce management is tricky!
    /// - This implementation always tries to fetch the latest on-chain nonce before sending a transaction,
    ///   and only falls back to the locally cached base_nonce if the sync fails.
    /// - Only the number of in-flight transactions is tracked locally.
    /// - For robust production use, consider also handling stuck transactions (e.g., gas bumping/EIP-1559).
    pub async fn create_raw_transaction(
        &self,
        payload: GolemBaseTransaction,
    ) -> Result<TransactionReceipt, Error> {
        log::debug!("payload: {:?}", payload);
        let mut buffer = Vec::new();
        payload.encode(&mut buffer);
        log::debug!("buffer: {:?}", buffer);
        let nonce = {
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
                    log::warn!("Failed to fetch on-chain nonce: {}", e);
                }
            }
            nm.next_nonce().await
        };
        let tx_base = TransactionRequest {
            to: Some(TxKind::Call(STORAGE_ADDRESS)),
            input: buffer.into(),
            chain_id: Some(
                self.provider
                    .get_chain_id()
                    .await
                    .map_err(|e| Error::TransactionSendError(e.to_string()))?,
            ),
            nonce: Some(nonce),
            ..Default::default()
        };
        log::debug!("transaction: {:?}", tx_base);
        let estimated_gas = self
            .provider
            .estimate_gas(tx_base.clone())
            .await
            .map_err(|e| Error::TransactionSendError(format!("Failed to estimate gas: {}", e)))?;
        let tx = tx_base.with_gas_limit(estimated_gas).with_gas_price(
            self.provider
                .get_gas_price()
                .await
                .map_err(|e| Error::TransactionSendError(e.to_string()))?,
        );
        let pending_tx = self
            .provider
            .send_transaction(tx)
            .await
            .map_err(|e| Error::TransactionSendError(e.to_string()))?;
        log::debug!("pending transaction: {:?}", pending_tx);
        let receipt = pending_tx
            .get_receipt()
            .await
            .map_err(|e| Error::TransactionReceiptError(e.to_string()))?;
        log::debug!("receipt: {:?}", receipt);
        {
            let mut nm = self.nonce_manager.lock().await;
            nm.complete().await;
        }
        Ok(receipt)
    }
}
