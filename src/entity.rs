use alloy::primitives::B256;
use alloy_rlp::{Encodable, RlpDecodable, RlpEncodable};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// A generic key-value pair structure.
#[derive(Debug, Clone, Serialize, Deserialize, RlpEncodable, RlpDecodable)]
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

/// A type alias for the hash used to identify entities in GolemBase.
pub type Hash = B256;

/// Type alias for the key used in annotations.
pub type Key = String;

/// Type representing a create transaction in GolemBase.
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct Create {
    /// The time-to-live (TTL) for the entity.
    pub ttl: u64,
    /// The data associated with the entity.
    pub data: Bytes,
    /// String annotations for the entity.
    pub string_annotations: Vec<StringAnnotation>,
    /// Numeric annotations for the entity.
    pub numeric_annotations: Vec<NumericAnnotation>,
}

/// Type representing an update transaction in GolemBase.
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct Update {
    /// The key of the entity to update.
    pub entity_key: Hash,
    /// The updated time-to-live (TTL) for the entity.
    pub ttl: u64,
    /// The updated data for the entity.
    pub data: Bytes,
    /// Updated string annotations for the entity.
    pub string_annotations: Vec<StringAnnotation>,
    /// Updated numeric annotations for the entity.
    pub numeric_annotations: Vec<NumericAnnotation>,
}

pub type GolemBaseDelete = Hash;

/// Type representing an extend transaction in GolemBase.
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable)]
pub struct Extend {
    /// The key of the entity to extend.
    pub entity_key: Hash,
    /// The number of blocks to extend the TTL by.
    pub number_of_blocks: u64,
}

/// Type representing a transaction in GolemBase, including creates, updates, deletes, and extensions.
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable)]
pub struct GolemBaseTransaction {
    /// A list of entities to create.
    pub creates: Vec<Create>,
    /// A list of entities to update.
    pub updates: Vec<Update>,
    /// A list of entity keys to delete.
    pub deletes: Vec<GolemBaseDelete>,
    /// A list of entities to extend.
    pub extensions: Vec<Extend>,
}

/// Represents an entity with data, TTL, and annotations.
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable)]
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

impl Create {
    /// Creates a new Create operation with empty annotations
    pub fn new(payload: Vec<u8>, ttl: u64) -> Self {
        Self {
            ttl,
            data: Bytes::from(payload),
            string_annotations: Vec::new(),
            numeric_annotations: Vec::new(),
        }
    }

    /// Adds a string annotation to the entity
    pub fn annotate_string(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.string_annotations.push(Annotation {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    /// Adds a numeric annotation to the entity
    pub fn annotate_number(mut self, key: impl Into<String>, value: u64) -> Self {
        self.numeric_annotations.push(Annotation {
            key: key.into(),
            value,
        });
        self
    }
}

impl Update {
    /// Creates a new Update operation with empty annotations
    pub fn new(entity_key: B256, payload: Vec<u8>, ttl: u64) -> Self {
        Self {
            entity_key,
            ttl,
            data: Bytes::from(payload),
            string_annotations: Vec::new(),
            numeric_annotations: Vec::new(),
        }
    }

    /// Adds a string annotation to the entity
    pub fn annotate_string(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.string_annotations.push(Annotation {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    /// Adds a numeric annotation to the entity
    pub fn annotate_number(mut self, key: impl Into<String>, value: u64) -> Self {
        self.numeric_annotations.push(Annotation {
            key: key.into(),
            value,
        });
        self
    }
}

impl GolemBaseTransaction {
    /// Returns the RLP-encoded bytes of the transaction
    pub fn encoded(&self) -> Vec<u8> {
        let mut encoded = Vec::new();
        self.encode(&mut encoded);
        encoded
    }
}

// Tests check serialization compatibility with go implementation.
#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::B256;
    use hex;

    #[test]
    fn test_empty_transaction() {
        let tx = GolemBaseTransaction::default();
        assert_eq!(hex::encode(tx.encoded()), "c4c0c0c0c0");
    }

    #[test]
    fn test_create_without_annotations() {
        let create = Create::new(b"test payload".to_vec(), 1000);

        let mut tx = GolemBaseTransaction::default();
        tx.creates.push(create);

        assert_eq!(
            hex::encode(tx.encoded()),
            "d7d3d28203e88c74657374207061796c6f6164c0c0c0c0c0"
        );
    }

    #[test]
    fn test_create_with_annotations() {
        let create = Create::new(b"test payload".to_vec(), 1000)
            .annotate_string("foo", "bar")
            .annotate_number("baz", 42);

        let mut tx = GolemBaseTransaction::default();
        tx.creates.push(create);

        assert_eq!(
            hex::encode(tx.encoded()),
            "e6e2e18203e88c74657374207061796c6f6164c9c883666f6f83626172c6c58362617a2ac0c0c0"
        );
    }

    #[test]
    fn test_update_with_annotations() {
        let update = Update::new(
            B256::from_slice(&[1; 32]),
            b"updated payload".to_vec(),
            2000,
        )
        .annotate_string("status", "active")
        .annotate_number("version", 2);

        let mut tx = GolemBaseTransaction::default();
        tx.updates.push(update);

        assert_eq!(
            hex::encode(tx.encoded()),
            "f856c0f851f84fa001010101010101010101010101010101010101010101010101010101010101018207d08f75706461746564207061796c6f6164cfce8673746174757386616374697665cac98776657273696f6e02c0c0"
        );
    }

    #[test]
    fn test_delete_operation() {
        let mut tx = GolemBaseTransaction::default();
        tx.deletes.push(B256::from_slice(&[2; 32]));

        assert_eq!(
            hex::encode(tx.encoded()),
            "e5c0c0e1a00202020202020202020202020202020202020202020202020202020202020202c0"
        );
    }

    #[test]
    fn test_extend_ttl() {
        let mut tx = GolemBaseTransaction::default();
        tx.extensions.push(Extend {
            entity_key: B256::from_slice(&[3; 32]),
            number_of_blocks: 500,
        });

        assert_eq!(
            hex::encode(tx.encoded()),
            "e9c0c0c0e5e4a003030303030303030303030303030303030303030303030303030303030303038201f4"
        );
    }

    #[test]
    fn test_mixed_operations() {
        let create = Create::new(b"test payload".to_vec(), 1000).annotate_string("type", "test");
        let update = Update::new(
            B256::from_slice(&[1; 32]),
            b"updated payload".to_vec(),
            2000,
        );
        let mut tx = GolemBaseTransaction::default();
        tx.creates.push(create);
        tx.updates.push(update);
        tx.deletes.push(B256::from_slice(&[2; 32]));
        tx.extensions.push(Extend {
            entity_key: B256::from_slice(&[3; 32]),
            number_of_blocks: 500,
        });

        assert_eq!(
            hex::encode(tx.encoded()),
            "f89fdedd8203e88c74657374207061796c6f6164cbca84747970658474657374c0f7f6a001010101010101010101010101010101010101010101010101010101010101018207d08f75706461746564207061796c6f6164c0c0e1a00202020202020202020202020202020202020202020202020202020202020202e5e4a003030303030303030303030303030303030303030303030303030303030303038201f4"
        );
    }
}
