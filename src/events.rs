use alloy::eips::BlockNumberOrTag;
use alloy::primitives::{keccak256, B256};
use alloy::providers::{DynProvider, Provider, ProviderBuilder, WsConnect};
use alloy::rpc::types::eth::Filter;
use alloy::rpc::types::Log;
use alloy::transports::http::reqwest::Url;
use anyhow::Result;
use futures::{Stream, StreamExt};
use std::convert::TryFrom;
use std::pin::Pin;

use crate::account::GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS;
use crate::entity::Hash;

/// Event signature for entity creation logs
pub fn golem_base_storage_entity_created() -> B256 {
    keccak256(b"GolemBaseStorageEntityCreated(uint256,uint256)")
}

/// Event signature for entity deletion logs
pub fn golem_base_storage_entity_deleted() -> B256 {
    keccak256(b"GolemBaseStorageEntityDeleted(uint256)")
}

/// Event signature for entity update logs
pub fn golem_base_storage_entity_updated() -> B256 {
    keccak256(b"GolemBaseStorageEntityUpdated(uint256,uint256)")
}

/// Event signature for extending TTL of an entity
pub fn golem_base_storage_entity_ttl_extended() -> B256 {
    keccak256(b"GolemBaseStorageEntityTTLExptended(uint256,uint256)")
}

/// Represents an event from the blockchain
#[derive(Debug)]
pub enum Event {
    /// Entity was created
    EntityCreated {
        /// The ID of the created entity
        entity_id: Hash,
        /// The block number where the event occurred
        block_number: u64,
        /// The transaction hash that triggered the event
        transaction_hash: Hash,
    },
    /// Entity was updated
    EntityUpdated {
        /// The ID of the updated entity
        entity_id: Hash,
        /// The block number where the event occurred
        block_number: u64,
        /// The transaction hash that triggered the event
        transaction_hash: Hash,
    },
    /// Entity was removed
    EntityRemoved {
        /// The ID of the removed entity
        entity_id: Hash,
        /// The block number where the event occurred
        block_number: u64,
        /// The transaction hash that triggered the event
        transaction_hash: Hash,
    },
}

impl TryFrom<Log> for Event {
    type Error = anyhow::Error;

    fn try_from(log: Log) -> Result<Self> {
        let block_number = log
            .block_number
            .ok_or_else(|| anyhow::anyhow!("Missing block number"))?;
        let transaction_hash = log
            .transaction_hash
            .ok_or_else(|| anyhow::anyhow!("Missing transaction hash"))?;

        if log.topics().len() < 2 {
            return Err(anyhow::anyhow!("Missing entity ID in event"));
        }

        let entity_id = Hash::from(log.topics()[1]);
        let transaction_hash = Hash::from(transaction_hash);

        match log.topics()[0] {
            topic if topic == golem_base_storage_entity_created() => Ok(Event::EntityCreated {
                entity_id,
                block_number,
                transaction_hash,
            }),
            topic if topic == golem_base_storage_entity_updated() => Ok(Event::EntityUpdated {
                entity_id,
                block_number,
                transaction_hash,
            }),
            topic if topic == golem_base_storage_entity_deleted() => Ok(Event::EntityRemoved {
                entity_id,
                block_number,
                transaction_hash,
            }),
            _ => Err(anyhow::anyhow!("Unknown event topic")),
        }
    }
}

/// Client for listening to GolemBase events
pub struct EventsClient {
    provider: DynProvider,
}

impl EventsClient {
    /// Creates a new EventsClient by connecting to the given URL
    pub async fn new(url: Url) -> anyhow::Result<Self> {
        let mut ws_url = url.clone();
        ws_url.set_scheme("ws").unwrap();

        let provider = ProviderBuilder::new()
            .connect_ws(WsConnect::new(ws_url))
            .await?
            .erased();

        Ok(Self { provider })
    }

    /// Listens for events from the blockchain
    /// Returns a stream of events that can be processed asynchronously
    pub async fn events_stream(
        &self,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<Event>> + Send>>> {
        let filter = Filter::new()
            .address(GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS)
            .from_block(BlockNumberOrTag::Latest)
            .event_signature(vec![
                golem_base_storage_entity_created(),
                golem_base_storage_entity_updated(),
                golem_base_storage_entity_deleted(),
            ]);

        let subscription = self.provider.subscribe_logs(&filter).await?;
        Ok(Box::pin(subscription.into_stream().map(Event::try_from)))
    }
}
