use std::ops::Deref;
use std::sync::Arc;

use alloy::eips::BlockNumberOrTag;
use alloy::primitives::Address;
use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use alloy::rpc::client::ClientRef;
use alloy::signers::local::PrivateKeySigner;
use alloy::transports::http::reqwest::Url;
use bigdecimal::BigDecimal;
use bon::bon;
use tokio::sync::Mutex;

use crate::utils::wei_to_eth;

/// Tracks and assigns sequential Ethereum nonces for concurrent transactions.
pub struct NonceManager {
    /// Last known on-chain nonce.
    pub base_nonce: u64,
    /// Number of in-flight (pending) transactions.
    pub in_flight: u64,
}

impl NonceManager {
    /// Returns the next available nonce and increments the in-flight counter.
    pub async fn next_nonce(&mut self) -> u64 {
        let nonce = self.base_nonce + self.in_flight;
        self.in_flight += 1;
        nonce
    }

    /// Marks a transaction as completed by decrementing the in-flight counter.
    pub async fn complete(&mut self) {
        if self.in_flight > 0 {
            self.in_flight -= 1;
        }
    }
}

/// A client for interacting with the GolemBase system.
/// Provides methods for account management, entity operations, balance queries, and event subscriptions.
///
/// # Example Usage
///
/// A client builder is provided for both [`GolemBaseClient`] and [`GolemBaseRoClient`],
/// however, an instance of [`GolemBaseClient`] can be dereferenced to [`GolemBaseRoClient`] like so:
///
/// ```rs
/// use golem_base_sdk::{GolemBaseClient, GolemBaseRoClient, PrivateKeySigner, Url};
///
/// let keypath = dirs::config_dir()
///     .ok_or("Failed to get config directory")?
///     .join("golembase")
///     .join("wallet.json");
/// let signer = PrivateKeySigner::decrypt_keystore(keypath, "password")?;
/// let url = Url::parse("http://localhost:8545")?;
///
/// let client = GolemBaseClient::builder()
///     .wallet(signer)
///     .rpc_url(url)
///     .build();
///
/// let ro_client: &GolemBaseRoClient = *client;
/// ```
#[derive(Clone)]
pub struct GolemBaseRoClient {
    /// The underlying provider for making RPC calls.
    pub(crate) provider: DynProvider,
}

#[bon]
impl GolemBaseRoClient {
    /// Creates a new builder for `GolemBaseClient` with the given wallet and RPC URL.
    /// Initializes the provider and sets up default configuration.
    #[builder]
    pub fn builder(rpc_url: Url, provider: Option<DynProvider>) -> Self {
        let provider = provider.unwrap_or_else(|| {
            ProviderBuilder::new()
                .connect_http(rpc_url.clone())
                .erased()
        });

        Self { provider }
    }
}

/// A client for interacting with the GolemBase system.
/// Provides methods for account management, entity operations, balance queries, and event subscriptions.
///
/// # Example Usage
///
/// A client builder is provided for both [`GolemBaseClient`] and [`GolemBaseRoClient`],
/// however, an instance of [`GolemBaseClient`] can be dereferenced to [`GolemBaseRoClient`] like so:
///
/// ```rs
/// use golem_base_sdk::{GolemBaseClient, GolemBaseRoClient, PrivateKeySigner, Url};
///
/// let keypath = dirs::config_dir()
///     .ok_or("Failed to get config directory")?
///     .join("golembase")
///     .join("wallet.json");
/// let signer = PrivateKeySigner::decrypt_keystore(keypath, "password")?;
/// let url = Url::parse("http://localhost:8545")?;
///
/// let client = GolemBaseClient::builder()
///     .wallet(signer)
///     .rpc_url(url)
///     .build();
///
/// let ro_client: &GolemBaseRoClient = *client;
/// ```
#[derive(Clone)]
pub struct GolemBaseClient {
    /// The underlying GolemBaseRoClient
    pub(crate) ro_client: GolemBaseRoClient,
    /// The Ethereum address of the client owner.
    pub(crate) wallet: PrivateKeySigner,
    /// Nonce manager for tracking transaction nonces.
    pub(crate) nonce_manager: Arc<Mutex<NonceManager>>,
}

impl Deref for GolemBaseClient {
    type Target = GolemBaseRoClient;

    fn deref(&self) -> &Self::Target {
        &self.ro_client
    }
}

#[bon]
impl GolemBaseClient {
    /// Creates a new builder for `GolemBaseClient` with the given wallet and RPC URL.
    /// Initializes the provider and sets up default configuration.
    #[builder]
    pub fn builder(wallet: PrivateKeySigner, rpc_url: Url) -> Self {
        let provider = ProviderBuilder::new()
            .wallet(wallet.clone())
            .connect_http(rpc_url.clone())
            .erased();

        let ro_client = GolemBaseRoClient::builder()
            .rpc_url(rpc_url)
            .provider(provider)
            .build();

        Self {
            ro_client,
            wallet,
            nonce_manager: Arc::new(Mutex::new(NonceManager {
                base_nonce: 0,
                in_flight: 0,
            })),
        }
    }

    /// Gets the underlying Reqwest client used for HTTP requests.
    pub fn get_reqwest_client(&self) -> ClientRef<'_> {
        self.provider.client()
    }

    /// Gets the Ethereum address of the client owner.
    pub fn get_owner_address(&self) -> Address {
        self.wallet.address()
    }

    /// Gets the chain ID from the provider.
    /// Returns the chain ID as a `u64`.
    pub async fn get_chain_id(&self) -> anyhow::Result<u64> {
        self.provider
            .get_chain_id()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get chain ID: {}", e))
    }

    /// Gets an account's ETH balance as a `BigDecimal`.
    pub async fn get_balance(&self, account: Address) -> anyhow::Result<BigDecimal> {
        let balance = self.provider.get_balance(account).await?;
        Ok(wei_to_eth(balance))
    }

    /// Gets the current block number from the chain.
    /// Returns the latest block number as a `u64`.
    pub async fn get_current_block_number(&self) -> anyhow::Result<u64> {
        let latest_block = self
            .provider
            .get_block_by_number(BlockNumberOrTag::Latest)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to get latest block"))?;
        Ok(latest_block.header.number)
    }
}
