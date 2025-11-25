use alloy_rlp::{RlpDecodable, RlpEncodable};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::entity::types::{
    attribute::{NumericAttribute, StringAttribute, WithAttribute},
    btl::BlocksToLive,
    content_type::ContentType,
};

use super::error::ValidationError;

/// Type representing part of a `Transaction` for creating a new `Entity`.
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
    /// Produces a builder to populate fields for a `Create` operation, as part
    /// of a `Transaction`. Requires `btl`, `content_type` and `payload` for
    /// `CreateBuilder::build` to succeed.
    pub fn builder<B, C, P>() -> CreateBuilder<B, C, P>
    where
        B: Into<BlocksToLive>,
        C: TryInto<ContentType<String>, Error = crate::entity::error::ValidationError>,
        P: Into<Bytes>,
    {
        CreateBuilder::new()
    }

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

/// Type representing part of a `Transaction` for creating a new `Entity`.
#[derive(Debug, Clone)]
pub struct CreateBuilder<B, C, P>
where
    B: Into<BlocksToLive>,
    C: TryInto<ContentType<String>, Error = crate::entity::error::ValidationError>,
    P: Into<Bytes>,
{
    /// The blocks-to-live (BTL) for the entity.
    btl: Option<B>,
    /// MIME type of the payload.
    content_type: Option<C>,
    /// The data associated with the entity.
    payload: Option<P>,
    /// String annotations of the entity.
    string_attributes: Vec<StringAttribute>,
    /// Numeric annotations of the entity.
    numeric_attributes: Vec<NumericAttribute>,
}
// Avoids the `Default` constraint on `Payload` and `Mime`.
impl<B, C, P> Default for CreateBuilder<B, C, P>
where
    B: Into<BlocksToLive>,
    C: TryInto<ContentType<String>, Error = crate::entity::error::ValidationError>,
    P: Into<Bytes>,
{
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
impl<B, C, P> CreateBuilder<B, C, P>
where
    B: Into<BlocksToLive>,
    C: TryInto<ContentType<String>, Error = crate::entity::error::ValidationError>,
    P: Into<Bytes>,
{
    pub fn new() -> Self {
        Default::default()
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

    pub fn build(self) -> Result<Create, ValidationError> {
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

        Ok(Create {
            btl,
            content_type: content_type.source().into(),
            payload,
            string_attributes: self.string_attributes,
            numeric_attributes: self.numeric_attributes,
        })
    }
}
impl<B, C, P> WithAttribute<StringAttribute> for CreateBuilder<B, C, P>
where
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
impl<B, C, P> WithAttribute<NumericAttribute> for CreateBuilder<B, C, P>
where
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

#[test]
fn test_create_builder() {
    use crate::entity::types::attribute::Attribute;

    const CONTENT_TYPE: ContentType<&str> =
        ContentType::new("application/json;mode=debug;version=1");
    const MODE: Attribute<&str, &str> = Attribute::new("mode", "debug");
    const VERS: Attribute<&str, u32> = Attribute::new("version", 1);

    let create = Create::builder()
        .btl(1000)
        .content_type(CONTENT_TYPE)
        // TODO: We should probably also test other serialization formats, like bincode,
        // if not just to have examples.
        .payload(serde_json::json!({ "key": "value" }).to_string())
        // Obviously not good, just pointing out that chaining maps is possible here
        // and testing that both methods compile.
        .with_attribute(MODE.map(StringAttribute::from).map_into::<String>())
        .with_attribute(VERS.map(NumericAttribute::from).map_into::<u64>())
        // A more idiomatic approach would be to provide an existing Vec or HashMap instead.
        .extend_attributes([
            StringAttribute::new("extend_str".into(), "value".into()),
            ("extend_str2".to_string(), "value2".to_string()).into(),
        ])
        .extend_attributes([
            NumericAttribute::new("extend_num".into(), 2),
            ("extend_num2".to_string(), 3u64).into(),
        ])
        .build();

    assert!(create.is_ok());
}

#[test]
fn test_with_bincode() {
    use bincode::{Decode, Encode};

    #[derive(Encode, Decode)]
    struct Payload;

    let mut payload = [0u8; 4];
    bincode::encode_into_slice(Payload, &mut payload, bincode::config::standard()).unwrap();

    let create = Create::builder()
        .btl(1000)
        .content_type("application/octet-stream")
        .payload(payload.to_vec()) // would be nice if Bytes implemented From<[u8; N]>...
        .build();

    assert!(create.is_ok());
}
