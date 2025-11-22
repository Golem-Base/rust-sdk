use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

pub type StringAttribute = Attribute<String, String>;
pub type NumericAttribute = Attribute<String, u64>;

/// A trait for attaching an attribute to a transaction.
///
/// Implementors provide distinct behavior depending on the wrapper type used
/// (e.g. string attributes vs numeric attributes).
pub trait WithAttribute<A> {
    fn with_attribute(self, attribute: A) -> Self;
    fn extend_attributes<I>(self, iter: I) -> Self
    where
        I: IntoIterator<Item = A>;
}

/// A generic key-value pair structure for entity attributes.
/// Used for both string and numeric metadata attached to entities.
#[derive(Debug, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Attribute<K: Into<String>, V> {
    /// The key of the attribute.
    key: K,
    /// The value of the attribute.
    value: V,
}

impl<K: Into<String>, V> Attribute<K, V> {
    /// Creates a new key-value pair attribute.
    /// Accepts a key which must implement `Into<String>` and a generic value.
    ///
    /// # Example
    ///
    /// ```rs,ignore
    /// use golem_base_sdk::entity::types::attribute::Attribute;
    ///
    /// const VERS: Attribute<&str, u64> = Attribute::new("version", 1u64);
    /// ```
    pub const fn new(key: K, value: V) -> Self {
        Self { key, value }
    }

    /// Provides a closure over `K` and `V` in order to convert
    /// an `Attribute` to another type `T`. This can be particularly
    /// useful for mapping old attributes to new ones.
    ///
    /// # Example
    ///
    /// ```rs,ignore
    /// use golem_base_sdk::entity::types::attribute::Attribute;
    ///
    /// const VERS: Attribute<&str, u32> = Attribute::new("version", 1);
    /// let mapped = VERS.map(NumericAttribute::from);
    /// ```
    pub fn map<F, T>(self, f: F) -> T
    where
        F: FnOnce((K, V)) -> T,
    {
        f((self.key, self.value))
    }

    /// A convenience wrapper for `Attribute::map` which converts
    /// `K` into a `String` and `V` into a type which implements
    /// `std::convert::From<V>`.
    ///
    /// # Example
    ///
    /// ```rs,ignore
    /// use golem_base_sdk::entity::types::attribute::Attribute;
    ///
    /// const MODE: Attribute<&str, &str> = Attribute::new("mode", "debug");
    /// let mapped = MODE.map_into::<String>();
    /// ```
    pub fn map_into<T>(self) -> Attribute<String, T>
    where
        T: From<V>,
    {
        self.map(|(key, value)| Attribute {
            key: key.into(),
            value: value.into(),
        })
    }

    /// The key of the attribute.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// The value of the attribute.
    pub fn value(&self) -> &V {
        &self.value
    }
}

impl<K: Into<String>, V> From<(K, V)> for Attribute<K, V> {
    fn from((key, value): (K, V)) -> Self {
        Self::new(key, value)
    }
}

impl<K: Into<String>> From<(K, &str)> for StringAttribute {
    fn from((key, value): (K, &str)) -> Self {
        StringAttribute::new(key.into(), value.into())
    }
}

impl<V: Into<u64>> From<(&str, V)> for NumericAttribute {
    fn from((key, value): (&str, V)) -> Self {
        NumericAttribute::new(key.into(), value.into())
    }
}
