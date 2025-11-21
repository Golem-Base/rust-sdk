use alloy::primitives::B256;
use alloy_rlp::{RlpDecodable, RlpEncodable};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::entity::{
    EntityKey,
    types::attribute::{Attribute, NumericAttributeValue, StringAttributeValue, WithAttribute},
};

/// Type representing an update transaction in GolemBase.
/// Used to update existing entities, including their data, BTL, and annotations.
///
/// > Note: Each block represents ~2 seconds, eg. setting the BTL (blocks-to-live) to
/// > `15u64` is equal to 30 seconds of life for the entity.
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
#[rlp(trailing)]
pub struct Update {
    /// The key of the entity to update.
    pub entity_key: EntityKey,
    /// The updated block-to-live (BTL) for the entity.
    pub btl: u64,
    /// The updated data for the entity.
    pub data: Bytes,
    /// Updated string annotations for the entity.
    pub string_attributes: Vec<Attribute<String>>,
    /// Updated numeric annotations for the entity.
    pub numeric_attributes: Vec<Attribute<u64>>,
}

impl Update {
    /// Creates a new `Update` operation with empty annotations.
    /// Accepts an entity key, payload as bytes, and a BTL value.
    pub fn new(entity_key: B256, payload: Vec<u8>, btl: u64) -> Self {
        Self {
            entity_key,
            btl,
            data: Bytes::from(payload),
            string_attributes: Vec::new(),
            numeric_attributes: Vec::new(),
        }
    }
}
impl<S> WithAttribute<StringAttributeValue<S>> for Update
where
    S: Into<String>,
{
    fn with_attribute<K: Into<String>, V: Into<StringAttributeValue<S>>>(
        mut self,
        key: K,
        value: V,
    ) -> Self {
        self.string_attributes.push(Attribute {
            key: key.into(),
            value: value.into().0.into(),
        });
        self
    }
}
impl WithAttribute<NumericAttributeValue> for Update {
    fn with_attribute<K: Into<String>, V: Into<NumericAttributeValue>>(
        mut self,
        key: K,
        value: V,
    ) -> Self {
        self.numeric_attributes.push(Attribute {
            key: key.into(),
            value: value.into().0,
        });
        self
    }
}
