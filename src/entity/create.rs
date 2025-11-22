use alloy_rlp::{RlpDecodable, RlpEncodable};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

pub use crate::entity::types::attribute::WithAttribute;
use crate::entity::types::{btl::BlocksToLive, content_type::ContentType};

use super::types::attribute::{NumericAttribute, StringAttribute};

/// Type representing a create transaction in GolemBase.
/// Used to define new entities, including their data, BTL, and attributes.
#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
#[rlp(trailing)]
pub struct Create {
    /// The blocks-to-live (BTL) for the entity.
    btl: BlocksToLive,
    /// MIME type of the payload.
    content_type: String,
    /// The data associated with the entity.
    payload: Bytes,
    /// String annotations of the entity.
    string_attributes: Vec<StringAttribute>,
    /// Numeric annotations of the entity.
    numeric_attributes: Vec<NumericAttribute>,
}
impl Create {
    /// The blocks-to-live (BTL) for the entity.
    pub fn btl(&self) -> BlocksToLive {
        self.btl
    }

    /// MIME type of the payload.
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// The data associated with the entity.
    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    /// String attributes of the entity.
    pub fn string_attributes(&self) -> &[StringAttribute] {
        &self.string_attributes
    }

    /// Numeric annotations of the entity.
    pub fn numeric_attributes(&self) -> &[NumericAttribute] {
        &self.numeric_attributes
    }
}

#[derive(Debug)]
pub struct CreateBuilder<Payload: Into<Bytes>> {
    btl: Option<BlocksToLive>,
    content_type: Option<ContentType>,
    payload: Option<Payload>,
    string_attributes: Vec<StringAttribute>,
    numeric_attributes: Vec<NumericAttribute>,
}

// Avoids the `Default` constraint on `Payload`.
impl<Payload: Into<Bytes>> Default for CreateBuilder<Payload> {
    fn default() -> Self {
        Self {
            btl: Default::default(),
            content_type: Default::default(),
            payload: Default::default(),
            string_attributes: Default::default(),
            numeric_attributes: Default::default(),
        }
    }
}

impl<Payload: Into<Bytes>> CreateBuilder<Payload> {
    pub fn new() -> Self {
        Default::default()
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

    pub fn build(self) -> Create {
        Create {
            btl: self.btl.unwrap(),
            content_type: self.content_type.unwrap().source().into(),
            payload: self.payload.unwrap().into(),
            string_attributes: self.string_attributes,
            numeric_attributes: self.numeric_attributes,
        }
    }
}
impl<Payload: Into<Bytes>> WithAttribute<StringAttribute> for CreateBuilder<Payload> {
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
impl<Payload: Into<Bytes>> WithAttribute<NumericAttribute> for CreateBuilder<Payload> {
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

#[test]
fn test_create_builder() {
    use crate::entity::types::attribute::Attribute;

    const MODE: Attribute<&str, &str> = Attribute::new("mode", "debug");
    const VERS: Attribute<&str, u32> = Attribute::new("version", 1);

    let create = CreateBuilder::default()
        .btl(1000.into())
        .content_type("application/json;mode=debug;version=1".try_into().unwrap())
        .payload(r#"{ "key": "value" }"#)
        // obviously not good, just pointing out that chaining maps is possible here
        // and testing that both methods compile.
        .with_attribute(MODE.map(StringAttribute::from).map_into::<String>())
        .with_attribute(VERS.map(NumericAttribute::from).map_into::<u64>())
        .extend_attributes([StringAttribute::new("extend_str".into(), "value".into())])
        .extend_attributes([NumericAttribute::new("extend_num".into(), 2)])
        .build();

    dbg!(&create);
}
