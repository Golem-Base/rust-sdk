use alloy_rlp::{RlpDecodable, RlpEncodable};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use super::{
    error::ValidationError,
    types::{
        BlocksToLive, ContentType,
        attribute::{NumericAttribute, StringAttribute, WithAttribute},
    },
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
impl Update {
    pub fn builder<K, B, C, P>() -> UpdateBuilder<K, B, C, P>
    where
        K: Into<EntityKey>,
        B: Into<BlocksToLive>,
        C: TryInto<ContentType<String>, Error = crate::entity::error::ValidationError>,
        P: Into<Bytes>,
    {
        UpdateBuilder::new()
    }

    pub fn entity_key(&self) -> &EntityKey {
        &self.entity_key
    }

    pub fn btl(&self) -> &BlocksToLive {
        &self.btl
    }

    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    pub fn string_attributes(&self) -> &[StringAttribute] {
        &self.string_attributes
    }

    pub fn numeric_attributes(&self) -> &[NumericAttribute] {
        &self.numeric_attributes
    }
}

#[derive(Debug)]
pub struct UpdateBuilder<K, B, C, P>
where
    K: Into<EntityKey>,
    B: Into<BlocksToLive>,
    C: TryInto<ContentType<String>, Error = crate::entity::error::ValidationError>,
    P: Into<Bytes>,
{
    entity_key: Option<K>,
    btl: Option<B>,
    content_type: Option<C>,
    payload: Option<P>,
    string_attributes: Vec<StringAttribute>,
    numeric_attributes: Vec<NumericAttribute>,
}

// Avoids the `Default` constraint on `Payload` and `Mime`.
impl<K, B, C, P> Default for UpdateBuilder<K, B, C, P>
where
    K: Into<EntityKey>,
    B: Into<BlocksToLive>,
    C: TryInto<ContentType<String>, Error = crate::entity::error::ValidationError>,
    P: Into<Bytes>,
{
    fn default() -> Self {
        Self {
            entity_key: Default::default(),
            btl: Default::default(),
            content_type: Default::default(),
            payload: Default::default(),
            string_attributes: Default::default(),
            numeric_attributes: Default::default(),
        }
    }
}

impl<K, B, C, P> UpdateBuilder<K, B, C, P>
where
    K: Into<EntityKey>,
    B: Into<BlocksToLive>,
    C: TryInto<ContentType<String>, Error = crate::entity::error::ValidationError>,
    P: Into<Bytes>,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn entity_key(mut self, entity_key: K) -> Self {
        self.entity_key = Some(entity_key);
        self
    }

    pub fn btl(mut self, btl: B) -> Self {
        self.btl = Some(btl);
        self
    }

    pub fn content_type(mut self, content_type: C) -> Self {
        self.content_type = Some(content_type);
        self
    }

    pub fn payload(mut self, payload: P) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn build(self) -> Result<Update, ValidationError> {
        let Some(entity_key) = self.entity_key.map(|key| key.into()) else {
            return Err(ValidationError::MissingEntityKey);
        };
        let Some(btl) = self.btl.map(|btl| btl.into()) else {
            return Err(ValidationError::MissingBtl);
        };
        let Some(content_type) = self.content_type else {
            return Err(ValidationError::MissingContentType);
        };
        let content_type = content_type.try_into()?;
        let Some(payload) = self.payload.map(Into::<Bytes>::into) else {
            return Err(ValidationError::MissingPayload);
        };

        Ok(Update {
            entity_key,
            btl,
            content_type: content_type.source().into(),
            payload,
            string_attributes: self.string_attributes,
            numeric_attributes: self.numeric_attributes,
        })
    }
}
impl<K, B, C, P> WithAttribute<StringAttribute> for UpdateBuilder<K, B, C, P>
where
    K: Into<EntityKey>,
    B: Into<BlocksToLive>,
    C: TryInto<ContentType<String>, Error = crate::entity::error::ValidationError>,
    P: Into<Bytes>,
{
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
impl<K, B, C, P> WithAttribute<NumericAttribute> for UpdateBuilder<K, B, C, P>
where
    K: Into<EntityKey>,
    B: Into<BlocksToLive>,
    C: TryInto<ContentType<String>, Error = crate::entity::error::ValidationError>,
    P: Into<Bytes>,
{
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
