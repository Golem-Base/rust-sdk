use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::rpc::json_rpc::RpcError;
use alloy::rpc::json_rpc::{RpcRecv, RpcSend};
use anyhow::anyhow;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use bytes::Bytes;
use displaydoc::Display;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::Debug;
use thiserror::Error;

use crate::{GolemBaseRoClient, Hash, NumericAnnotation, StringAnnotation};

/// Represents errors that can occur in the GolemBase RPC module.
/// Used to wrap and describe errors from RPC requests, decoding, or deserialization.
#[derive(Debug, Display, Error)]
pub enum Error {
    /// Failed to send the RPC request: {0}
    RpcRequestError(String),
    /// Failed to decode the base64-encoded storage value: {0}
    Base64DecodeError(String),
    /// Failed to deserialize the RPC response: {0}
    ResponseDeserializationError(String),
    /// Unexpected error occurred: {0}
    UnexpectedError(String),
}

/// Type representing metadata for an entity.
/// Contains information such as expiration, payload, annotations, and owner.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityMetaData {
    /// The block number at which the entity expires.
    pub expires_at_block: Option<u64>,
    /// The payload associated with the entity.
    pub payload: Option<String>,
    /// String annotations for the entity.
    pub string_annotations: Vec<StringAnnotation>,
    /// Numeric annotations for the entity.
    pub numeric_annotations: Vec<NumericAnnotation>,
    /// The owner of the entity.
    pub owner: Address,
}

/// Represents a single search result from a query.
/// Contains the entity key and the value (decoded from base64).
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    #[serde(rename = "key")]
    pub key: Hash,
    #[serde(rename = "value", deserialize_with = "deserialize_base64")]
    pub value: Bytes,
}

/// Helper for deserializing base64-encoded storage values.
/// Used to decode entity values returned from the RPC API.
fn deserialize_base64<'de, D>(deserializer: D) -> Result<Bytes, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    BASE64
        .decode(s)
        .map(Bytes::from)
        .map_err(serde::de::Error::custom)
}

impl SearchResult {
    /// Converts the value to a UTF-8 string.
    /// Returns an error if the value is not valid UTF-8.
    pub fn value_as_string(&self) -> anyhow::Result<String> {
        String::from_utf8(self.value.to_vec())
            .map_err(|e| anyhow::anyhow!("Failed to decode search result to string: {}", e))
    }
}

impl GolemBaseRoClient {
    /// Makes a JSON-RPC call to the GolemBase endpoint.
    /// Handles serialization, deserialization, and error mapping for RPC requests.
    pub(crate) async fn rpc_call<S: RpcSend, R: RpcRecv>(
        &self,
        method: impl Into<Cow<'static, str>>,
        params: S,
    ) -> Result<R, Error> {
        let method = method.into();
        tracing::debug!("RPC Call - Method: {method}, Params: {params:?}");
        self.provider
            .client()
            .request(method.clone(), params)
            .await
            .inspect(|res| tracing::debug!("RPC Response: {res:?}"))
            .map_err(|e| match e {
                RpcError::ErrorResp(err) => {
                    anyhow!("Error response from RPC service: {err}")
                }
                RpcError::SerError(err) => {
                    anyhow!("Serialization error: {err}")
                }
                RpcError::DeserError { err, text } => {
                    tracing::debug!("Deserialization error: {err}, response text: {text}");
                    anyhow!("Deserialization error: {err}")
                }
                _ => anyhow!("{e}"),
            })
            .map_err(|e| Error::RpcRequestError(e.to_string()))
    }

    /// Gets the total count of entities in GolemBase.
    /// Returns the number of entities currently stored.
    pub async fn get_entity_count(&self) -> Result<u64, Error> {
        self.rpc_call::<(), u64>("golembase_getEntityCount", ())
            .await
    }

    /// Gets the entity keys of all entities in GolemBase.
    /// Returns a vector of all entity keys.
    pub async fn get_all_entity_keys(&self) -> Result<Vec<Hash>, Error> {
        let result = self
            .rpc_call::<(), Option<Vec<Hash>>>("golembase_getAllEntityKeys", ())
            .await?;
        Ok(result.unwrap_or_default())
    }

    /// Gets the entity keys of all entities owned by the given address.
    /// Returns a vector of entity keys for the specified owner.
    pub async fn get_entities_of_owner(&self, address: Address) -> Result<Vec<Hash>, Error> {
        let result = self
            .rpc_call::<&[Address], Option<Vec<Hash>>>("golembase_getEntitiesOfOwner", &[address])
            .await?;
        Ok(result.unwrap_or_default())
    }

    /// Gets the storage value associated with the given entity key.
    /// Decodes the value from base64 and attempts to convert it to the requested type.
    pub async fn get_storage_value<T: TryFrom<Vec<u8>>>(&self, key: Hash) -> Result<T, Error>
    where
        <T as TryFrom<Vec<u8>>>::Error: std::fmt::Display,
    {
        let encoded_value = self
            .rpc_call::<&[Hash], String>("golembase_getStorageValue", &[key])
            .await?;
        let decoded = BASE64
            .decode(&encoded_value)
            .map_err(|e| Error::Base64DecodeError(e.to_string()))?;
        T::try_from(decoded).map_err(|e| Error::UnexpectedError(e.to_string()))
    }

    /// Queries entities in GolemBase based on annotations.
    /// Returns a vector of `SearchResult` matching the query string.
    pub async fn query_entities(&self, query: &str) -> Result<Vec<SearchResult>, Error> {
        let results = self
            .rpc_call::<&[&str], Option<Vec<SearchResult>>>("golembase_queryEntities", &[query])
            .await?;
        // GolemBase returns null if no entities are found. Option is used to correctly
        // deserialize this value in Rust, since Vec<_> expects empty array [].
        Ok(results.unwrap_or_default())
    }

    /// Queries entities in GolemBase based on annotations and returns only their keys.
    /// Returns a vector of entity keys matching the query string.
    pub async fn query_entity_keys(&self, query: &str) -> Result<Vec<Hash>, Error> {
        let results = self.query_entities(query).await?;
        Ok(results.into_iter().map(|result| result.key).collect())
    }

    /// Gets all entity keys for entities that will expire at the given block number.
    /// Returns a vector of entity keys expiring at the specified block.
    pub async fn get_entities_to_expire_at_block(
        &self,
        block_number: u64,
    ) -> Result<Vec<Hash>, Error> {
        let result = self
            .rpc_call::<u64, Option<Vec<Hash>>>(
                "golembase_getEntitiesToExpireAtBlock",
                block_number,
            )
            .await?;
        Ok(result.unwrap_or_default())
    }

    /// Gets metadata for a specific entity.
    /// Returns an `EntityMetaData` struct for the given entity key.
    pub async fn get_entity_metadata(&self, key: Hash) -> Result<EntityMetaData, Error> {
        self.rpc_call::<&[Hash], EntityMetaData>("golembase_getEntityMetaData", &[key])
            .await
    }
}
