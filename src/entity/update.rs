use alloy_rlp::{RlpDecodable, RlpEncodable};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use super::types::{
    BlocksToLive, ContentType,
    attribute::{NumericAttribute, StringAttribute, WithAttribute},
};
use crate::entity::EntityKey;

/// Type representing an update transaction in GolemBase.
/// Used to update existing entities, including their data, BTL, and annotations.
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
#[rlp(trailing)]
pub struct Update {
    /// The key of the entity to update.
    entity_key: EntityKey,
    /// The updated block-to-live (BTL) for the entity.
    btl: BlocksToLive,
    /// MIME type of the payload.
    content_type: String,
    /// The updated data for the entity.
    payload: Bytes,
    /// Updated string annotations for the entity.
    string_attributes: Vec<StringAttribute>,
    /// Updated numeric annotations for the entity.
    numeric_attributes: Vec<NumericAttribute>,
}

#[derive(Debug, Default)]
pub struct UpdateBuilder<Payload: Into<Bytes> + Default> {
    entity_key: Option<EntityKey>,
    btl: Option<BlocksToLive>,
    content_type: Option<ContentType>,
    payload: Option<Payload>,
    string_attributes: Vec<StringAttribute>,
    numeric_attributes: Vec<NumericAttribute>,
}

impl<Payload: Into<Bytes> + Default> UpdateBuilder<Payload> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn entity_key(mut self, entity_key: EntityKey) -> Self {
        self.entity_key = Some(entity_key);
        self
    }

    pub fn btl(mut self, btl: BlocksToLive) -> Self {
        self.btl = Some(btl);
        self
    }

    pub fn content_type(mut self, content_type: ContentType) -> Self {
        self.content_type = Some(content_type);
        self
    }

    pub fn payload(mut self, payload: Payload) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn build(self) -> Update {
        Update {
            entity_key: self.entity_key.unwrap(),
            btl: self.btl.unwrap(),
            content_type: self.content_type.unwrap().source().into(),
            payload: self.payload.unwrap().into(),
            string_attributes: self.string_attributes,
            numeric_attributes: self.numeric_attributes,
        }
    }
}
impl<Payload: Into<Bytes> + Default> WithAttribute<StringAttribute> for UpdateBuilder<Payload> {
    fn with_attribute(mut self, attribute: StringAttribute) -> Self {
        self.string_attributes.push(attribute);
        self
    }
    fn extend_attributes<I>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = StringAttribute>,
    {
        self.string_attributes.extend(iter);
        self
    }
}
impl<Payload: Into<Bytes> + Default> WithAttribute<NumericAttribute> for UpdateBuilder<Payload> {
    fn with_attribute(mut self, attribute: NumericAttribute) -> Self {
        self.numeric_attributes.push(attribute);
        self
    }
    fn extend_attributes<I>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = NumericAttribute>,
    {
        self.numeric_attributes.extend(iter);
        self
    }
}
