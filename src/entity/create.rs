use alloy_rlp::{RlpDecodable, RlpEncodable};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::entity::{
    error::ValidationError,
    types::{
        attribute::{Attribute, NumericAttributeValue, StringAttributeValue, WithAttribute},
        btl::BlocksToLive,
        content_type::ContentType,
    },
};

// TODO: Use builder pattern for transaction types
/// Type representing a create transaction in GolemBase.
/// Used to define new entities, including their data, BTL, and attributes.
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
#[rlp(trailing)]
pub struct Create {
    /// The blocks-to-live (BTL) for the entity.
    pub btl: BlocksToLive,
    /// MIME type of the payload.
    pub content_type: String,
    /// The data associated with the entity.
    pub payload: Bytes,
    /// String annotations for the entity.
    pub string_attributes: Vec<Attribute<String>>,
    /// Numeric annotations for the entity.
    pub numeric_attributes: Vec<Attribute<u64>>,
}
impl Create {
    /// Creates a new `Create` operation with empty annotations.
    /// Accepts a payload as bytes and a BTL value.
    pub fn new<Payload>(
        content_type: ContentType,
        payload: Payload,
        btl: BlocksToLive,
    ) -> Result<Self, ValidationError>
    where
        Payload: Into<Bytes>,
    {
        Ok(Self {
            btl,
            content_type: content_type.source().to_string(),
            payload: payload.into(),
            ..Default::default()
        })
    }
}
impl<S> WithAttribute<StringAttributeValue<S>> for Create
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
impl WithAttribute<NumericAttributeValue> for Create {
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
