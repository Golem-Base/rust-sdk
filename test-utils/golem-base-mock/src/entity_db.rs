use alloy::primitives::{keccak256, Address, B256};
use bytes::Bytes;
use golem_base_sdk::entity::{Create, Update};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Represents a GolemBase entity
#[derive(Clone, Debug)]
pub struct Entity {
    pub key: B256,
    pub data: Bytes,
    pub btl: u64,
    pub owner: Address,
    pub expires_at: Option<u64>,
    pub string_annotations: Vec<String>,
    pub numeric_annotations: Vec<u64>,
}

impl Entity {
    /// Create a new entity from a GolemBase Create object and owner
    pub fn create(create: Create, owner: Address) -> Self {
        let entity = Self {
            key: B256::ZERO, // Temporary key, will be calculated below
            data: create.data.clone(),
            btl: create.btl,
            owner,
            expires_at: None, // Will be computed based on current block + BTL
            string_annotations: create
                .string_annotations
                .iter()
                .map(|a| a.key.clone())
                .collect(),
            numeric_annotations: create.numeric_annotations.iter().map(|a| a.value).collect(),
        };

        entity
    }

    /// Calculate the hash of this entity based on its content, owner, block number, index and transaction hash
    /// This creates a deterministic identifier that combines entity content with block context
    pub fn calculate_hash(&self, block_number: u64, idx: usize, transaction_hash: B256) -> B256 {
        let mut content_bytes = Vec::new();
        content_bytes.extend_from_slice(&self.data);
        content_bytes.extend_from_slice(self.btl.to_le_bytes().as_slice());
        content_bytes.extend_from_slice(self.owner.as_slice());
        content_bytes.extend_from_slice(block_number.to_le_bytes().as_slice());
        content_bytes.extend_from_slice(idx.to_le_bytes().as_slice());
        content_bytes.extend_from_slice(transaction_hash.as_slice());

        for annotation in &self.string_annotations {
            content_bytes.extend_from_slice(annotation.as_bytes());
        }

        for annotation in &self.numeric_annotations {
            content_bytes.extend_from_slice(annotation.to_le_bytes().as_slice());
        }

        keccak256(&content_bytes)
    }

    /// Set the entity key to a hash based on block number, index and transaction hash
    /// This modifies the entity in place and returns self for chaining
    pub fn with_hash(mut self, block_number: u64, idx: usize, transaction_hash: B256) -> Self {
        self.key = self.calculate_hash(block_number, idx, transaction_hash);
        self
    }

    /// Update this entity with data from a GolemBase Update object
    /// This modifies the entity in place and returns self for chaining
    pub fn update(&mut self, update: &Update) -> &mut Self {
        self.data = update.data.clone();
        self.btl = update.btl;
        self.string_annotations = update
            .string_annotations
            .iter()
            .map(|a| a.key.clone())
            .collect();
        self.numeric_annotations = update.numeric_annotations.iter().map(|a| a.value).collect();
        self
    }
}

/// Internal state of the entity database
#[derive(Clone, Debug, Default)]
struct EntityDbState {
    entities: HashMap<B256, Entity>,
    string_annotations: HashMap<String, Vec<B256>>,
    numeric_annotations: HashMap<u64, Vec<B256>>,
}

/// GolemBase entity database
#[derive(Clone, Debug, Default)]
pub struct EntityDb {
    state: Arc<RwLock<EntityDbState>>,
}

impl EntityDb {
    /// Create a new empty entity database
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(EntityDbState {
                entities: HashMap::new(),
                string_annotations: HashMap::new(),
                numeric_annotations: HashMap::new(),
            })),
        }
    }

    /// Add an entity to the database
    pub async fn add_entity(&self, entity: Entity) {
        let key = entity.key;
        let mut state = self.state.write().await;

        // Add to main entities map
        state.entities.insert(key, entity.clone());

        // Update annotation indices
        Self::update_annotations(
            &mut state,
            entity.key,
            &entity.string_annotations,
            &entity.numeric_annotations,
        );
    }

    /// Update an existing entity in the database with new data
    /// Returns true if entity was updated, false if entity doesn't exist
    pub async fn update_entity(&self, entity_key: &B256, update: &Update) -> bool {
        let mut state = self.state.write().await;

        if let Some(entity) = state.entities.get_mut(entity_key) {
            // Update the entity using the existing update method
            entity.update(update);

            // Get entity data to avoid borrowing conflicts
            let entity_key = entity.key;
            let new_string_annotations = entity.string_annotations.clone();
            let new_numeric_annotations = entity.numeric_annotations.clone();

            // Update annotation indices
            Self::update_annotations(
                &mut state,
                entity_key,
                &new_string_annotations,
                &new_numeric_annotations,
            );

            true
        } else {
            // Entity doesn't exist, ignore the update
            false
        }
    }

    /// Update only the BTL of an existing entity
    pub async fn update_entity_btl(&self, entity_key: &B256, new_btl: u64) {
        let mut state = self.state.write().await;

        if let Some(entity) = state.entities.get_mut(entity_key) {
            entity.btl = new_btl;
        }
    }

    /// Function to remove all annotations for a specific entity
    fn remove_entity_annotations(state: &mut EntityDbState, entity_key: B256) {
        // Remove all existing annotations for this entity from hash maps
        for (_, keys) in state.string_annotations.iter_mut() {
            keys.retain(|&k| k != entity_key);
        }
        for (_, keys) in state.numeric_annotations.iter_mut() {
            keys.retain(|&k| k != entity_key);
        }
    }

    /// Unified function to update annotation indices
    /// Removes all existing annotations for the entity and adds new ones
    fn update_annotations(
        state: &mut EntityDbState,
        entity_key: B256,
        new_string_annotations: &[String],
        new_numeric_annotations: &[u64],
    ) {
        // Remove all existing annotations first
        Self::remove_entity_annotations(state, entity_key);

        // Add new annotations to hash maps
        for annotation in new_string_annotations {
            state
                .string_annotations
                .entry(annotation.clone())
                .or_insert_with(Vec::new)
                .push(entity_key);
        }
        for annotation in new_numeric_annotations {
            state
                .numeric_annotations
                .entry(*annotation)
                .or_insert_with(Vec::new)
                .push(entity_key);
        }
    }

    /// Get an entity by its key
    pub async fn get_entity(&self, key: &B256) -> Option<Entity> {
        self.state.read().await.entities.get(key).cloned()
    }

    /// Get entities by string annotation
    pub async fn by_string_annotation(&self, annotation: &str) -> Vec<Entity> {
        let state = self.state.read().await;
        let keys = state
            .string_annotations
            .get(annotation)
            .cloned()
            .unwrap_or_default();
        let mut entities = Vec::new();

        for key in keys {
            if let Some(entity) = state.entities.get(&key) {
                entities.push(entity.clone());
            }
        }

        entities
    }

    /// Get entities by numeric annotation
    pub async fn by_numeric_annotation(&self, annotation: u64) -> Vec<Entity> {
        let state = self.state.read().await;
        let keys = state
            .numeric_annotations
            .get(&annotation)
            .cloned()
            .unwrap_or_default();
        let mut entities = Vec::new();

        for key in keys {
            if let Some(entity) = state.entities.get(&key) {
                entities.push(entity.clone());
            }
        }

        entities
    }

    /// Remove an entity from the database
    pub async fn remove_entity(&self, key: &B256) -> Option<Entity> {
        let mut state = self.state.write().await;
        if let Some(entity) = state.entities.remove(key) {
            // Remove all annotations for this entity
            Self::remove_entity_annotations(&mut state, entity.key);
            Some(entity)
        } else {
            None
        }
    }

    /// Get all entity keys
    pub async fn get_all_keys(&self) -> Vec<B256> {
        self.state.read().await.entities.keys().cloned().collect()
    }

    /// Get total number of entities
    pub async fn count(&self) -> usize {
        self.state.read().await.entities.len()
    }
}
