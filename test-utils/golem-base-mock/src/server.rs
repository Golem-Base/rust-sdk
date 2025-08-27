use alloy::primitives::{Address, U256};
use anyhow;
use jsonrpsee::server::{RpcModule, Server};
use std::net::SocketAddr;
use url::Url;

use crate::api::{EthRpcServer, GolemBaseRpcServer};
use crate::controller::MockController;
use crate::GolemBaseMock;

/// GolemBase Mock Server
#[derive(Clone)]
pub struct GolemBaseMockServer {
    pub state: GolemBaseMock,
    pub url: Url,
    #[allow(dead_code)]
    server: Option<jsonrpsee::server::ServerHandle>,
}

impl GolemBaseMockServer {
    pub fn new() -> Self {
        Self {
            state: GolemBaseMock::new(),
            url: Url::parse("http://127.0.0.1:8585").unwrap(),
            server: None,
        }
    }

    pub fn controller(&self) -> &MockController {
        &self.state.controller
    }

    pub fn with_chain_id(mut self, chain_id: u64) -> Self {
        self.state.chain_id = U256::from(chain_id);
        self
    }

    pub async fn create_test_account(&mut self, initial_balance: U256) -> Address {
        let address = self.state.create_account();
        self.state.blockchain.add_accounts(vec![address]).await;
        self.state
            .blockchain
            .set_balance(address, initial_balance)
            .await;
        address
    }

    pub async fn start(self, addr: SocketAddr) -> anyhow::Result<Self> {
        let mut module = RpcModule::new(());

        // Register RPC methods (both Ethereum and GolemBase)
        let rpc_impl = self.state.clone();
        module.merge(EthRpcServer::into_rpc(rpc_impl.clone()))?;
        module.merge(GolemBaseRpcServer::into_rpc(rpc_impl))?;

        let server = Server::builder().build(addr).await?;

        let actual_addr = server.local_addr()?;
        log::info!("GolemBase Mock Server listening on {}", actual_addr);

        // Start the execution engine to produce blocks
        self.state.blockchain.create_genesis_block().await;
        self.state.execution.start().await;

        let server_handle = server.start(module);

        Ok(Self {
            state: self.state,
            url: self.url,
            server: Some(server_handle),
        })
    }

    /// Get the server URL
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Convert URL to SocketAddr
    pub fn socket_addr(&self) -> anyhow::Result<SocketAddr> {
        let url = self.url.clone();
        let host = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("URL has no host"))?;
        let port = url
            .port()
            .ok_or_else(|| anyhow::anyhow!("URL has no port specified"))?;

        let addr = format!("{}:{}", host, port);
        addr.parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse socket address: {}", e))
    }

    /// Create a test mock server with test accounts and balances
    pub async fn create_test_mock_server() -> anyhow::Result<Self> {
        // Create server
        let mut server = Self::new().with_chain_id(1337);

        // Create test accounts with initial balance
        server
            .create_test_account(U256::from(1000000000000000000000u128))
            .await;

        // Start the server
        let socket_addr = server.socket_addr()?;
        let server = server.start(socket_addr).await?;

        Ok(server)
    }
}
