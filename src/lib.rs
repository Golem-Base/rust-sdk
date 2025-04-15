// Re-export commonly used types from `alloy`.
pub use alloy::primitives::Address;
pub use alloy::signers::local::PrivateKeySigner;
pub use alloy::transports::http::reqwest::Url;

use alloy::primitives::B256;
use alloy::rpc::client::{ClientBuilder, ReqwestClient, RpcClient};
use alloy_rlp::RlpEncodable;
use bon::bon;
use serde::{Deserialize, Serialize};

/// Module for Ethereum transaction-related functionality.
pub mod eth;

/// Module for JSON-RPC-related functionality.
pub mod rpc;

/// Type alias for string annotations.
pub type StringAnnotation = Annotation<String>;

/// Type alias for numeric annotations.
pub type NumericAnnotation = Annotation<u64>;

/// A type alias for the hash used to identify entities in GolemBase.
pub type Hash = B256;

/// Type alias for the key used in annotations.
pub type Key = String;

/// A generic key-value pair structure.
#[derive(Debug, Clone, RlpEncodable, Serialize, Deserialize)]
pub struct Annotation<T> {
    /// The key of the annotation.
    pub key: Key,
    /// The value of the annotation.
    pub value: T,
}

impl<T> Annotation<T> {
    /// Creates a new key-value pair.
    pub fn new<K, V>(key: K, value: V) -> Self
    where
        K: Into<Key>,
        V: Into<T>,
    {
        Annotation {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// A client for interacting with the GolemBase system.
pub struct GolemBaseClient {
    /// The underlying HTTP client used for JSON-RPC requests.
    client: ReqwestClient,
    /// The signer used for signing Ethereum transactions.
    wallet: PrivateKeySigner,
    /// The URL of the GolemBase JSON-RPC endpoint.
    url: Url,
}

#[bon]
impl GolemBaseClient {
    /// Creates a new GolemBase client.
    pub fn new(wallet: PrivateKeySigner, rpc_url: Url) -> Self {
        let client = ClientBuilder::default().http(rpc_url.clone());
        GolemBaseClient {
            client,
            wallet,
            url: rpc_url,
        }
    }

    #[builder]
    pub fn builder(wallet: PrivateKeySigner, rpc_url: Url) -> Self {
        Self::new(wallet, rpc_url)
    }

    /// Gets the Ethereum address of the client owner.
    pub fn get_owner_address(&self) -> Address {
        self.wallet.address()
    }

    /// Gets the underlying Reqwest client used for HTTP requests.
    pub fn get_reqwest_client(&self) -> &RpcClient {
        &self.client
    }
}
