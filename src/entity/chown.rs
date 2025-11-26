use alloy::primitives::Address;
use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::entity::EntityKey;

/// Type representing a change of ownership operation as part of a `Transaction`.
/// Sender must be the current owner and the entity must exist. Metadata other than
/// the current owner is preserved.
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Chown {
    /// The key of the entity to transfer.
    entity_key: EntityKey,
    /// The address to transfer ownership of the entity to.
    new_owner: Address,
}
impl Chown {
    /// Construct a new instance of an change owner operation as part of a `Transaction`
    /// for some existing [`crate::entity::Entity`] and address.
    pub fn new<K: Into<EntityKey>, A: Into<Address>>(entity_key: K, new_owner: A) -> Self {
        Self {
            entity_key: entity_key.into(),
            new_owner: new_owner.into(),
        }
    }
}

/// Represents the result of changing ownership of an entity.
pub struct ChownResult {
    /// The key of the entity that was transferred.
    pub entity_key: EntityKey,
    /// The previous owner of the entity.
    pub prev_owner: Address,
    /// The current owner of the entity.
    pub curr_owner: Address,
}
