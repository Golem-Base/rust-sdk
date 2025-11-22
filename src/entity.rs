use alloy::primitives::B256;
use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use types::attribute::{NumericAttribute, StringAttribute};

pub mod chown;
pub mod create;
pub mod delete;
pub mod error;
pub mod extend;
pub mod tx;
pub mod types;
pub mod update;

use crate::entity::types::btl::BlocksToLive;

/// Represents an entity with data, BTL, and annotations.
/// Used for reading entity state from the chain.
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Entity {
    /// The data associated with the entity.
    pub data: String,
    /// The block-to-live (BTL) for the entity.
    pub btl: BlocksToLive,
    /// String annotations for the entity.
    pub string_attributes: Vec<StringAttribute>,
    /// Numeric annotations for the entity.
    pub numeric_attributes: Vec<NumericAttribute>,
}

/// Represents the result of creating or updating an entity.
/// Contains the entity key and its expiration block.
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct EntityResult {
    /// The key of the entity.
    pub entity_key: EntityKey,
    /// The block number at which the entity expires.
    pub expiration_block: u64,
}

/// A type alias for the hash used to identify entities in GolemBase.
pub type EntityKey = B256;

// Tests check serialization compatibility with go implementation.
#[cfg(test)]
mod serialization_tests {
    use alloy::primitives::B256;
    use hex;

    use crate::entity::{
        create::Create, extend::Extend, tx::GolemBaseTransaction, types::attribute::WithAttribute,
        update::Update,
    };

    #[test]
    fn test_empty_transaction() {
        let tx = GolemBaseTransaction::builder().build();
        assert_eq!(hex::encode(tx.encoded()), "c4c0c0c0c0");
    }

    #[test]
    fn test_create_without_annotations() {
        let create = crate::entity::create::Create::new(
            "application/json".try_into().unwrap(),
            b"test payload".to_vec(),
            1000.into(),
        )
        .unwrap();

        let tx = GolemBaseTransaction::builder()
            .creates(vec![create])
            .build();

        assert_eq!(
            hex::encode(tx.encoded()),
            "d7d3d28203e88c74657374207061796c6f6164c0c0c0c0c0"
        );
    }

    #[test]
    fn test_create_with_annotations() {
        let create = crate::entity::create::Create::new(
            "application/json".try_into().unwrap(),
            b"test payload".to_vec(),
            1000.into(),
        )
        .unwrap()
        .with_attribute("foo", "bar")
        .with_attribute("baz", 42);

        let tx = GolemBaseTransaction::builder()
            .creates(vec![create])
            .build();

        assert_eq!(
            hex::encode(tx.encoded()),
            "e6e2e18203e88c74657374207061796c6f6164c9c883666f6f83626172c6c58362617a2ac0c0c0"
        );
    }

    #[test]
    fn test_update_with_annotations() {
        let update = crate::entity::update::Update::new(
            B256::from_slice(&[1; 32]),
            b"updated payload".to_vec(),
            2000,
        )
        .with_attribute("status", "active")
        .with_attribute("version", 2);

        let tx = GolemBaseTransaction::builder()
            .updates(vec![update])
            .build();

        assert_eq!(
            hex::encode(tx.encoded()),
            "f856c0f851f84fa001010101010101010101010101010101010101010101010101010101010101018207d08f75706461746564207061796c6f6164cfce8673746174757386616374697665cac98776657273696f6e02c0c0"
        );
    }

    #[test]
    fn test_delete_operation() {
        let tx = GolemBaseTransaction::builder()
            .deletes(vec![B256::from_slice(&[2; 32]).into()])
            .build();

        assert_eq!(
            hex::encode(tx.encoded()),
            "e5c0c0e1a00202020202020202020202020202020202020202020202020202020202020202c0"
        );
    }

    #[test]
    fn test_extend_btl() {
        let tx = GolemBaseTransaction::builder()
            .extensions(vec![Extend {
                entity_key: B256::from_slice(&[3; 32]),
                number_of_blocks: 500,
            }])
            .build();

        assert_eq!(
            hex::encode(tx.encoded()),
            "e9c0c0c0e5e4a003030303030303030303030303030303030303030303030303030303030303038201f4"
        );
    }

    #[test]
    fn test_mixed_operations() {
        let create = Create::new(
            "application/json".try_into().unwrap(),
            b"test payload".to_vec(),
            1000.into(),
        )
        .unwrap()
        .with_attribute("type", "test");
        let update = Update::new(
            B256::from_slice(&[1; 32]),
            b"updated payload".to_vec(),
            2000,
        );
        let tx = GolemBaseTransaction::builder()
            .creates(vec![create])
            .updates(vec![update])
            .deletes(vec![B256::from_slice(&[2; 32]).into()])
            .extensions(vec![Extend {
                entity_key: B256::from_slice(&[3; 32]),
                number_of_blocks: 500,
            }])
            .build();

        assert_eq!(
            hex::encode(tx.encoded()),
            "f89fdedd8203e88c74657374207061796c6f6164cbca84747970658474657374c0f7f6a001010101010101010101010101010101010101010101010101010101010101018207d08f75706461746564207061796c6f6164c0c0e1a00202020202020202020202020202020202020202020202020202020202020202e5e4a003030303030303030303030303030303030303030303030303030303030303038201f4"
        );
    }
}
