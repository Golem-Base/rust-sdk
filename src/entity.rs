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
    use expect_test::expect;
    use hex;

    use crate::entity::{
        create::Create,
        extend::Extend,
        tx::Transaction,
        types::attribute::{NumericAttribute, StringAttribute, WithAttribute},
        update::Update,
    };

    pub fn expect_hex(hex: &str, expect: expect_test::Expect) {
        expect.assert_eq(hex);
    }

    #[test]
    fn test_empty_transaction() {
        let tx = Transaction::builder().build();
        expect_hex(&hex::encode(tx.encoded()), expect!["c4c0c0c0c0"]);
    }

    #[test]
    fn test_create_without_annotations() {
        let create = Create::builder()
            .btl(1000)
            .content_type("application/json")
            .payload(serde_json::json!({ "test": "payload" }).to_string())
            .build()
            .unwrap();

        let tx = Transaction::builder().creates(vec![create]).build();

        expect_hex(
            &hex::encode(tx.encoded()),
            expect![
                "efebeac38203e8906170706c69636174696f6e2f6a736f6e927b2274657374223a227061796c6f6164227dc0c0c0c0c0"
            ],
        );
    }

    #[test]
    fn test_create_with_annotations() {
        let create = Create::builder()
            .btl(1000)
            .content_type("application/json")
            .payload(serde_json::json!({ "test": "payload" }).to_string())
            .with_attribute(StringAttribute::from(("foo", "bar")))
            .with_attribute(NumericAttribute::from(("baz", 42u64)))
            .build()
            .unwrap();

        let tx = Transaction::builder().creates(vec![create]).build();

        expect_hex(
            &hex::encode(tx.encoded()),
            expect![
                "f840f83bf839c38203e8906170706c69636174696f6e2f6a736f6e927b2274657374223a227061796c6f6164227dc9c883666f6f83626172c6c58362617a2ac0c0c0"
            ],
        );
    }

    #[test]
    fn test_update_with_annotations() {
        let update = Update::builder()
            .entity_key(&[1; 32])
            .content_type("plain/text")
            .payload(b"updated payload".to_vec())
            .btl(2000)
            .with_attribute(StringAttribute::new("status".into(), "active".into()))
            .with_attribute(NumericAttribute::new("version".into(), 2))
            .build()
            .unwrap();

        let tx = Transaction::builder().updates(vec![update]).build();

        expect_hex(
            &hex::encode(tx.encoded()),
            expect![
                "f862c0f85df85ba00101010101010101010101010101010101010101010101010101010101010101c38207d08a706c61696e2f746578748f75706461746564207061796c6f6164cfce8673746174757386616374697665cac98776657273696f6e02c0c0"
            ],
        );
    }

    #[test]
    fn test_delete_operation() {
        let tx = Transaction::builder()
            .deletes(vec![B256::from_slice(&[2; 32]).into()])
            .build();

        expect_hex(
            &hex::encode(tx.encoded()),
            expect![
                "e6c0c0e2e1a00202020202020202020202020202020202020202020202020202020202020202c0"
            ],
        );
    }

    #[test]
    fn test_extend_btl() {
        let tx = Transaction::builder()
            .extensions(vec![Extend::new(&[3; 32], 500)])
            .build();

        expect_hex(
            &hex::encode(tx.encoded()),
            expect![
                "e9c0c0c0e5e4a003030303030303030303030303030303030303030303030303030303030303038201f4"
            ],
        );
    }

    #[test]
    fn test_mixed_operations() {
        let create = Create::builder()
            .content_type("plain/text")
            .payload("test payload")
            .btl(1000)
            .with_attribute(StringAttribute::new("type".to_string(), "test".to_string()))
            .build()
            .unwrap();
        let update = Update::builder()
            .entity_key(&[1; 32])
            .content_type("plain/text")
            .payload(b"updated payload".to_vec())
            .btl(2000)
            .build()
            .unwrap();
        let tx = Transaction::builder()
            .creates(vec![create])
            .updates(vec![update])
            .deletes(vec![B256::from_slice(&[2; 32]).into()])
            .extensions(vec![Extend::new(&[3; 32], 500)])
            .build();

        expect_hex(
            &hex::encode(tx.encoded()),
            expect![
                "f8baeae9c38203e88a706c61696e2f746578748c74657374207061796c6f6164cbca84747970658474657374c0f844f842a00101010101010101010101010101010101010101010101010101010101010101c38207d08a706c61696e2f746578748f75706461746564207061796c6f6164c0c0e2e1a00202020202020202020202020202020202020202020202020202020202020202e5e4a003030303030303030303030303030303030303030303030303030303030303038201f4"
            ],
        );
    }
}
