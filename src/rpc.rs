use crate::{GolemBaseClient, Hash, NumericAnnotation, StringAnnotation};
use alloy::primitives::Address;
use alloy::rpc::json_rpc::{RpcRecv, RpcSend};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use displaydoc::Display;
use log::debug;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::Debug;
use thiserror::Error;

/// Represents errors that can occur in the GolemBase RPC module.
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

impl GolemBaseClient {
    /// Makes a JSON-RPC call to the GolemBase endpoint.
    pub(crate) async fn rpc_call<S: RpcSend, R: RpcRecv>(
        &self,
        method: impl Into<Cow<'static, str>>,
        params: S,
    ) -> Result<R, Error> {
        let method = method.into();
        let request = self.client.request(method.clone(), params.clone());
        match request.await {
            Ok(response) => {
                debug!("{}({:?}), response: {:?}", method, params, response);
                Ok(response)
            }
            Err(err) => {
                debug!("{}({:?}), error: {:?}", method, params, err);
                Err(Error::RpcRequestError(err.to_string()))
            }
        }
    }

    /// Gets the total count of entities in GolemBase.
    pub async fn get_entity_count(&self) -> Result<u64, Error> {
        self.rpc_call::<(), u64>("golembase_getEntityCount", ())
            .await
    }

    /// Gets the entity keys of all entities in GolemBase.
    pub async fn get_all_entity_keys(&self) -> Result<Vec<Hash>, Error> {
        self.rpc_call::<(), Vec<Hash>>("golembase_getAllEntityKeys", ())
            .await
    }

    /// Gets the entity keys of all entities owned by the given address.
    pub async fn get_entities_of_owner(&self, address: Address) -> Result<Vec<Hash>, Error> {
        self.rpc_call::<&[Address], Vec<Hash>>("golembase_getEntitiesOfOwner", &[address])
            .await
    }

    /// Gets the storage value associated with the given entity key.
    pub async fn get_storage_value<T: TryFrom<Vec<u8>>>(&self, key: Hash) -> Result<T, Error>
    where
        <T as TryFrom<Vec<u8>>>::Error: std::fmt::Display,
    {
        let encoded_value = self
            .rpc_call::<&[Hash], String>("golembase_getStorageValue", &[key])
            .await?;
        let decoded = STANDARD
            .decode(&encoded_value)
            .map_err(|e| Error::Base64DecodeError(e.to_string()))?;
        T::try_from(decoded).map_err(|e| Error::UnexpectedError(e.to_string()))
    }

    /// Queries entities in GolemBase based on annotations.
    pub async fn query_entities(&self, query: &str) -> Result<Vec<Hash>, Error> {
        #[derive(Debug, Deserialize)]
        struct ReturnType {
            pub key: Hash,
            #[allow(dead_code)]
            pub value: String,
        }
        let results = self
            .rpc_call::<&[&str], Vec<ReturnType>>("golembase_queryEntities", &[query])
            .await?;
        Ok(results.into_iter().map(|result| result.key).collect())
    }

    /// Gets all entity keys for entities that will expire at the given block number.
    pub async fn get_entities_to_expire_at_block(
        &self,
        block_number: u64,
    ) -> Result<Vec<Hash>, Error> {
        self.rpc_call::<u64, Vec<Hash>>("golembase_getEntitiesToExpireAtBlock", block_number)
            .await
    }

    /// Gets metadata for a specific entity.
    pub async fn get_entity_metadata(&self, key: Hash) -> Result<EntityMetaData, Error> {
        self.rpc_call::<&[Hash], EntityMetaData>("golembase_getEntityMetaData", &[key])
            .await
    }
}
