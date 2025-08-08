use alloy::primitives::B256;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Represents a GolemBase entity
#[derive(Clone, Debug)]
pub struct Entity {
    pub key: B256,
    pub data: Bytes,
    pub btl: u64,
    pub owner: B256,
    pub expires_at: Option<u64>,
    pub string_annotations: Vec<String>,
    pub numeric_annotations: Vec<u64>,
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

        // Add to string annotations index
        for annotation in &entity.string_annotations {
            state
                .string_annotations
                .entry(annotation.clone())
                .or_insert_with(Vec::new)
                .push(key);
        }

        // Add to numeric annotations index
        for annotation in &entity.numeric_annotations {
            state
                .numeric_annotations
                .entry(*annotation)
                .or_insert_with(Vec::new)
                .push(key);
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
            // Remove from string annotations index
            for annotation in &entity.string_annotations {
                if let Some(keys) = state.string_annotations.get_mut(annotation) {
                    keys.retain(|&k| k != *key);
                }
            }

            // Remove from numeric annotations index
            for annotation in &entity.numeric_annotations {
                if let Some(keys) = state.numeric_annotations.get_mut(annotation) {
                    keys.retain(|&k| k != *key);
                }
            }

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
