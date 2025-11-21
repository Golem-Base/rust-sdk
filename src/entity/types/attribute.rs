use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

use crate::entity::create::Create;

/// A generic key-value pair structure for entity attributes.
/// Used for both string and numeric metadata attached to entities.
#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Attribute<T> {
    /// The key of the annotation.
    pub key: String,
    /// The value of the annotation.
    pub value: T,
}

impl<T> Attribute<T> {
    /// Creates a new key-value pair attribute.
    /// Accepts any types convertible to `Key` and the value.
    pub fn new<K, V>(key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<T>,
    {
        Attribute {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// A wrapper type used to distinguish string-like attribute values from any
/// other type that implements `Into<String>`.
///
/// This prevents blanket `Into<String>` implementations (such as those coming
/// from `Display`) from causing trait resolution conflicts when used with the
/// `WithAttribute` trait.
///
/// By wrapping the underlying type `S`, we create a unique type that can be
/// used in trait implementations without overlapping with implementations for
/// other attribute value types.
///
/// End users will not need to construct this type directly, and instead can
/// pass `S` to functions which expect `Into<StringAttributeValue<S>>`.
#[derive(Debug, Clone)]
pub(crate) struct StringAttributeValue<S>(pub(crate) S)
where
    S: Into<String>;
impl<S> From<S> for StringAttributeValue<S>
where
    S: Into<String>,
{
    fn from(value: S) -> Self {
        Self(value)
    }
}

/// A wrapper type for numeric attribute values. This exists for the same reason
/// as `StringAttributeValue`: to provide a type-level distinction that avoids
/// trait overlap when implementing `WithAttribute` for different attribute
/// value kinds.
///
/// This prevents blanket `Into<String>` implementations (such as those coming
/// from `Display`) from causing trait resolution conflicts when used with the
/// `WithAttribute` trait.
///
/// By wrapping the underlying type `u64`, we create a unique type that can be
/// used in trait implementations without overlapping with implementations for
/// other attribute value types.
///
/// End users will not need to construct this type directly, and instead can
/// pass `u64` to functions which expect `Into<NumericAttribute>`.
pub(crate) struct NumericAttributeValue(pub(crate) u64);
impl From<u64> for NumericAttributeValue {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

/// A trait for attaching an attribute to an object. The attribute key is any
/// type convertible into `String`, and the attribute value type `V` determines
/// which implementation applies.
///
/// Implementors provide distinct behavior depending on the wrapper type used
/// (e.g. string attributes vs numeric attributes).
pub trait WithAttribute<A> {
    fn with_attribute<K: Into<String>, V: Into<A>>(self, key: K, value: V) -> Self;
}
