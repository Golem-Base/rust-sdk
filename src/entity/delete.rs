use crate::entity::EntityKey;

/// Type representing a delete transaction in GolemBase.
pub struct GolemBaseDelete(EntityKey);
impl GolemBaseDelete {
    pub fn new(entity_key: EntityKey) -> Self {
        Self(entity_key)
    }
    pub fn entity_key(&self) -> &EntityKey {
        &self.0
    }
}

/// Represents the result of deleting an entity.
/// Contains the key of the deleted entity.
#[derive(Debug)]
pub struct DeleteResult {
    /// The key of the entity that was deleted.
    pub entity_key: EntityKey,
}
