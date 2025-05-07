use crate::GolemBaseClient;
use alloy::primitives::{address, Address, TxKind, B256};
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;
use alloy::rpc::types::{Log, TransactionReceipt, TransactionRequest};
use alloy_rlp::{Encodable, RlpEncodable};
use displaydoc::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Represents errors that can occur in the GolemBase ETH client.
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
pub const STORAGE_ADDRESS: Address = address!("0x0000000000000000000000000000000060138453");

/// The chain ID for the GolemBase Ethereum network.
pub const CHAIN_ID: u64 = 1337;

/// A type alias for the hash used to identify entities in GolemBase.
pub type Hash = B256;

/// Type alias for the key used in annotations.
pub type Key = String;

/// A generic key-value pair structure.
#[derive(Debug, Clone, RlpEncodable, Serialize, Deserialize)]
pub struct Annotation<T> {
    /// The key of the annotation.
    pub key: Key,
    /// The value of the annotation.
    pub value: T,
}

impl<T> Annotation<T> {
    /// Creates a new key-value pair.
    pub fn new<K, V>(key: K, value: V) -> Self
    where
        K: Into<Key>,
        V: Into<T>,
    {
        Annotation {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// Type alias for string annotations.
pub type StringAnnotation = Annotation<String>;

/// Type alias for numeric annotations.
pub type NumericAnnotation = Annotation<u64>;

/// Type representing a create transaction in GolemBase.
#[derive(Debug, RlpEncodable)]
pub struct GolemBaseCreate {
    /// The data associated with the entity.
    pub data: String,
    /// The time-to-live (TTL) for the entity.
    pub ttl: u64,
    /// String annotations for the entity.
    pub string_annotations: Vec<StringAnnotation>,
    /// Numeric annotations for the entity.
    pub numeric_annotations: Vec<NumericAnnotation>,
}

/// Type representing an update transaction in GolemBase.
#[derive(Debug, RlpEncodable)]
pub struct GolemBaseUpdate {
    /// The key of the entity to update.
    pub entity_key: Hash,
    /// The updated data for the entity.
    pub data: String,
    /// The updated time-to-live (TTL) for the entity.
    pub ttl: u64,
    /// Updated string annotations for the entity.
    pub string_annotations: Vec<StringAnnotation>,
    /// Updated numeric annotations for the entity.
    pub numeric_annotations: Vec<NumericAnnotation>,
}

pub type GolemBaseDelete = Hash;

/// Type representing an extend transaction in GolemBase.
#[derive(Debug, RlpEncodable)]
pub struct GolemBaseExtend {
    /// The key of the entity to extend.
    pub entity_key: Hash,
    /// The number of blocks to extend the TTL by.
    pub number_of_blocks: u64,
}

/// Type representing a transaction in GolemBase, including creates, updates, deletes, and extensions.
#[derive(Debug, RlpEncodable)]
pub struct GolemBaseTransaction {
    /// A list of entities to create.
    pub creates: Vec<GolemBaseCreate>,
    /// A list of entities to update.
    pub updates: Vec<GolemBaseUpdate>,
    /// A list of entity keys to delete.
    pub deletes: Vec<GolemBaseDelete>,
    /// A list of entities to extend.
    pub extensions: Vec<GolemBaseExtend>,
}

/// Represents an entity with data, TTL, and annotations.
#[derive(Debug)]
pub struct Entity {
    /// The data associated with the entity.
    pub data: String,
    /// The time-to-live (TTL) for the entity.
    pub ttl: u64,
    /// String annotations for the entity.
    pub string_annotations: Vec<StringAnnotation>,
    /// Numeric annotations for the entity.
    pub numeric_annotations: Vec<NumericAnnotation>,
}

/// Represents the result of creating or updating an entity.
#[derive(Debug)]
pub struct EntityResult {
    /// The key of the entity.
    pub entity_key: Hash,
    /// The block number at which the entity expires.
    pub expiration_block: u64,
}

/// Represents the result of extending an entity's TTL.
#[derive(Debug)]
pub struct ExtendResult {
    /// The key of the entity.
    pub entity_key: Hash,
    /// The old expiration block of the entity.
    pub old_expiration_block: u64,
    /// The new expiration block of the entity.
    pub new_expiration_block: u64,
}

/// Represents the result of deleting an entity.
#[derive(Debug)]
pub struct DeleteResult {
    /// The key of the entity that was deleted.
    pub entity_key: Hash,
}

impl GolemBaseClient {
    /// Creates one or more new entities in GolemBase and returns their results.
    pub async fn create_entities(
        &self,
        creates: Vec<GolemBaseCreate>,
    ) -> Result<Vec<EntityResult>, Error> {
        let receipt = self
            .create_raw_transaction(GolemBaseTransaction {
                creates,
                updates: vec![],
                deletes: vec![],
                extensions: vec![],
            })
            .await?;
        self.process_receipt(receipt, |log| {
            if log.topics().len() < 2 {
                return None;
            }
            let expiration_block = Self::parse_expiration_block(log.data().data.as_ref());
            Some(EntityResult {
                entity_key: log.topics()[1],
                expiration_block,
            })
        })
        .await
    }

    /// Updates one or more entities in GolemBase and returns their results.
    pub async fn update_entities(
        &self,
        updates: Vec<GolemBaseUpdate>,
    ) -> Result<Vec<EntityResult>, Error> {
        let receipt = self
            .create_raw_transaction(GolemBaseTransaction {
                creates: vec![],
                updates,
                deletes: vec![],
                extensions: vec![],
            })
            .await?;
        self.process_receipt(receipt, |log| {
            if log.topics().len() < 2 {
                return None;
            }
            let expiration_block = Self::parse_expiration_block(log.data().data.as_ref());
            Some(EntityResult {
                entity_key: log.topics()[1],
                expiration_block,
            })
        })
        .await
    }

    /// Deletes one or more entities in GolemBase and returns their results.
    pub async fn delete_entities(&self, deletes: Vec<Hash>) -> Result<Vec<DeleteResult>, Error> {
        let receipt = self
            .create_raw_transaction(GolemBaseTransaction {
                creates: vec![],
                updates: vec![],
                deletes,
                extensions: vec![],
            })
            .await?;
        self.process_receipt(receipt, |log| {
            if log.topics().len() < 2 {
                return None;
            }
            Some(DeleteResult {
                entity_key: log.topics()[1],
            })
        })
        .await
    }

    /// Extends the TTL of one or more entities in GolemBase and returns their results.
    pub async fn extend_entities(
        &self,
        extensions: Vec<GolemBaseExtend>,
    ) -> Result<Vec<ExtendResult>, Error> {
        let receipt = self
            .create_raw_transaction(GolemBaseTransaction {
                creates: vec![],
                updates: vec![],
                deletes: vec![],
                extensions,
            })
            .await?;
        self.process_receipt(receipt, |log| {
            let data = log.data().data.as_ref();
            if log.topics().len() < 2 {
                return None;
            }
            let old_expiration_block = Self::parse_expiration_block(&data[..8]);
            let new_expiration_block = Self::parse_expiration_block(&data[8..]);
            Some(ExtendResult {
                entity_key: log.topics()[1],
                old_expiration_block,
                new_expiration_block,
            })
        })
        .await
    }

    /// Creates and sends a raw transaction to the GolemBase storage contract.
    pub async fn create_raw_transaction(
        &self,
        payload: GolemBaseTransaction,
    ) -> Result<TransactionReceipt, Error> {
        log::debug!("payload: {:?}", payload);
        let mut buffer = Vec::new();
        payload.encode(&mut buffer);
        log::debug!("buffer: {:?}", buffer);
        let tx = TransactionRequest {
            to: Some(TxKind::Call(STORAGE_ADDRESS)),
            input: buffer.into(),
            chain_id: Some(CHAIN_ID),
            ..Default::default()
        };
        log::debug!("transaction: {:?}", tx);
        let provider = ProviderBuilder::new()
            .wallet(self.wallet.clone())
            .connect_http(self.url.clone());
        log::debug!("provider: {:?}", provider);
        let pending_tx = provider
            .send_transaction(tx)
            .await
            .map_err(|e| Error::TransactionSendError(e.to_string()))?;
        log::debug!("pending transaction: {:?}", pending_tx);
        let receipt = pending_tx
            .get_receipt()
            .await
            .map_err(|e| Error::TransactionReceiptError(e.to_string()))?;
        log::debug!("receipt: {:?}", receipt);
        Ok(receipt)
    }

    /// Processes a transaction receipt and maps logs into the desired result type.
    async fn process_receipt<T, F>(
        &self,
        receipt: TransactionReceipt,
        log_mapper: F,
    ) -> Result<Vec<T>, Error>
    where
        F: Fn(&Log) -> Option<T>,
    {
        let results: Vec<T> = receipt
            .logs()
            .iter()
            .filter(|log| log.address() == STORAGE_ADDRESS)
            .filter_map(log_mapper)
            .collect();
        Ok(results)
    }

    /// Parses a single `u64` value from log data, padding the beginning with zeros if needed.
    fn parse_expiration_block(data: &[u8]) -> u64 {
        let mut padded_data = [0u8; 8];
        let start = 8_usize.saturating_sub(data.len());
        padded_data[start..].copy_from_slice(&data[..data.len().min(8)]);
        u64::from_be_bytes(padded_data)
    }
}
