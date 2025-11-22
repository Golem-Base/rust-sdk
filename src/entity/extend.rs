use alloy::primitives::B256;
use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::entity::EntityKey;

/// Type representing an extend transaction in GolemBase.
/// Used to extend the BTL of an entity by a number of blocks.
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Extend {
    /// The key of the entity to extend.
    pub entity_key: EntityKey,
    /// The number of blocks to extend the BTL by.
    pub number_of_blocks: u64,
}

impl Extend {
    /// Creates a new `Update` operation with empty annotations.
    /// Accepts an entity key, payload as bytes, and a BTL value.
    pub fn new(entity_key: B256, number_of_blocks: u64) -> Self {
        Self {
            entity_key,
            number_of_blocks,
        }
    }
}

/// Represents the result of extending an entity's BTL.
/// Contains the entity key, old expiration block, and new expiration block.
#[derive(Debug)]
pub struct ExtendResult {
    /// The key of the entity.
    pub entity_key: EntityKey,
    /// The old expiration block of the entity.
    pub old_expiration_block: u64,
    /// The new expiration block of the entity.
    pub new_expiration_block: u64,
}
