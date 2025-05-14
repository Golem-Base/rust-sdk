use alloy::primitives::{keccak256, B256};
use alloy::rpc::types::Log;
use anyhow::Result;
use std::convert::TryFrom;

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
