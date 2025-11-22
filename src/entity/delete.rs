use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::entity::EntityKey;

/// Type representing a delete transaction in GolemBase.
#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Delete(EntityKey);
impl Delete {
    pub fn new(entity_key: EntityKey) -> Self {
        Self(entity_key)
    }
    pub fn entity_key(&self) -> &EntityKey {
        &self.0
    }
}
impl From<EntityKey> for Delete {
    fn from(value: EntityKey) -> Self {
        Self(value)
    }
}

/// Represents the result of deleting an entity.
/// Contains the key of the deleted entity.
#[derive(Debug)]
pub struct DeleteResult {
    /// The key of the entity that was deleted.
    pub entity_key: EntityKey,
}
