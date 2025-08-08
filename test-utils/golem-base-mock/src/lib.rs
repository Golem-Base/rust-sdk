use alloy::primitives::{Address, B256, U256};
use alloy::rpc::types::{Block, BlockId, BlockNumberOrTag, Transaction, TransactionReceipt};
use anyhow::Result;
use jsonrpsee::core::{async_trait, RpcResult};
use jsonrpsee::server::{RpcModule, Server};
use rand::Rng;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::{EthRpcServer, GolemBaseRpcServer};

pub mod api;
pub mod blockchain;
pub mod entity_db;
pub mod execution;
pub mod server;
pub mod transaction_pool;

// Re-export server functions for convenience
pub use server::{create_test_mock_server, get_default_mock_server_url};

/// Internal state data for the mock server
#[derive(Clone, Default)]
struct MockStateData {
    chain_id: U256,
    accounts: Vec<Address>,
    balances: HashMap<Address, U256>,
    transaction_count: HashMap<Address, U256>,
    transactions: HashMap<B256, Transaction>,
    receipts: HashMap<B256, TransactionReceipt>,
    blocks: HashMap<u64, Block>,
    entities: HashMap<B256, serde_json::Value>,
    syncing: bool,
}

impl MockStateData {
    fn new() -> Self {
        Self {
            chain_id: U256::from(1337),
            ..Default::default()
        }
    }
}

/// Mock state for the RPC server
#[derive(Clone, Default)]
pub struct MockState {
    data: Arc<RwLock<MockStateData>>,
}

/// Mock implementation of RPC methods (both Ethereum and GolemBase)
#[derive(Clone)]
pub struct GolemBaseMock {
    state: MockState,
}

impl GolemBaseMock {
    pub fn new(state: MockState) -> Self {
        Self { state }
    }
}
#[async_trait]
impl EthRpcServer for GolemBaseMock {
    async fn get_transaction_count(
        &self,
        address: Address,
        _block: Option<BlockId>,
    ) -> RpcResult<U256> {
        let count = self
            .state
            .data
            .read()
            .await
            .transaction_count
            .get(&address)
            .copied()
            .unwrap_or(U256::ZERO);
        Ok(count)
    }

    async fn get_transaction_receipt(&self, hash: B256) -> RpcResult<Option<TransactionReceipt>> {
        let receipt = self.state.data.read().await.receipts.get(&hash).cloned();
        Ok(receipt)
    }

    async fn get_proof(
        &self,
        _address: Address,
        _keys: Vec<B256>,
        _block: Option<BlockId>,
    ) -> RpcResult<serde_json::Value> {
        // Mock implementation - return empty proof
        Ok(serde_json::json!({
            "address": "0x0000000000000000000000000000000000000000",
            "accountProof": [],
            "balance": "0x0",
            "codeHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "nonce": "0x0",
            "storageHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "storageProof": []
        }))
    }

    async fn get_balance(&self, address: Address, _block: Option<BlockId>) -> RpcResult<U256> {
        let balance = self
            .state
            .data
            .read()
            .await
            .balances
            .get(&address)
            .copied()
            .unwrap_or(U256::ZERO);
        Ok(balance)
    }

    async fn accounts(&self) -> RpcResult<Vec<Address>> {
        Ok(self.state.data.read().await.accounts.clone())
    }

    async fn get_accounts(&self) -> RpcResult<Vec<Address>> {
        Ok(self.state.data.read().await.accounts.clone())
    }

    async fn send_transaction(&self, _transaction: serde_json::Value) -> RpcResult<B256> {
        // Mock implementation - return a random transaction hash
        let mut rng = rand::thread_rng();
        let tx_hash = B256::new(rng.gen());
        Ok(tx_hash)
    }

    async fn send_raw_transaction(&self, _data: String) -> RpcResult<B256> {
        // Mock implementation - return a random transaction hash
        let mut rng = rand::thread_rng();
        let tx_hash = B256::new(rng.gen());
        Ok(tx_hash)
    }

    async fn chain_id(&self) -> RpcResult<U256> {
        Ok(self.state.data.read().await.chain_id)
    }

    async fn get_transaction_by_hash(&self, hash: B256) -> RpcResult<Option<Transaction>> {
        let transaction = self
            .state
            .data
            .read()
            .await
            .transactions
            .get(&hash)
            .cloned();
        Ok(transaction)
    }

    async fn syncing(&self) -> RpcResult<bool> {
        Ok(self.state.data.read().await.syncing)
    }

    async fn get_block_by_number(
        &self,
        block: BlockNumberOrTag,
        _full: Option<bool>,
    ) -> RpcResult<Option<Block>> {
        match block {
            BlockNumberOrTag::Number(block_num) => {
                let block = self.state.data.read().await.blocks.get(&block_num).cloned();
                Ok(block)
            }
            BlockNumberOrTag::Latest => {
                // Return the latest block or None
                let blocks = &self.state.data.read().await.blocks;
                let latest_block = blocks
                    .keys()
                    .max()
                    .and_then(|&num| blocks.get(&num).cloned());
                Ok(latest_block)
            }
            _ => Ok(None),
        }
    }
}

#[async_trait]
impl GolemBaseRpcServer for GolemBaseMock {
    async fn get_entity(&self, _key: B256) -> RpcResult<Option<serde_json::Value>> {
        // Mock implementation - return None for now
        Ok(None)
    }

    async fn search(&self, _query: serde_json::Value) -> RpcResult<Vec<serde_json::Value>> {
        // Mock implementation - return empty search results
        Ok(vec![])
    }

    async fn get_entity_metadata(&self, _key: B256) -> RpcResult<Option<serde_json::Value>> {
        // Mock implementation - return empty metadata
        Ok(Some(serde_json::json!({
            "expires_at_block": null,
            "payload": null,
            "string_annotations": [],
            "numeric_annotations": [],
            "owner": "0x0000000000000000000000000000000000000000"
        })))
    }

    async fn get_entity_count(&self) -> RpcResult<u64> {
        // Mock implementation - return 0 for now
        Ok(0)
    }

    async fn get_all_entity_keys(&self) -> RpcResult<Option<Vec<B256>>> {
        // Mock implementation - return empty list
        Ok(Some(vec![]))
    }

    async fn get_entities_of_owner(
        &self,
        _addresses: Vec<Address>,
    ) -> RpcResult<Option<Vec<B256>>> {
        // Mock implementation - return empty list
        Ok(Some(vec![]))
    }

    async fn get_storage_value(&self, _keys: Vec<B256>) -> RpcResult<String> {
        // Mock implementation - return empty base64 encoded string
        Ok("".to_string())
    }

    async fn query_entities(
        &self,
        _queries: Vec<String>,
    ) -> RpcResult<Option<Vec<serde_json::Value>>> {
        // Mock implementation - return empty list
        Ok(Some(vec![]))
    }

    async fn get_entities_to_expire_at_block(
        &self,
        _block_number: u64,
    ) -> RpcResult<Option<Vec<B256>>> {
        // Mock implementation - return empty list
        Ok(Some(vec![]))
    }
}

/// GolemBase Mock Server
pub struct GolemBaseMockServer {
    state: MockState,
    server: Option<jsonrpsee::server::ServerHandle>,
}

impl GolemBaseMockServer {
    pub fn new() -> Self {
        Self {
            state: MockState::default(),
            server: None,
        }
    }

    pub fn with_chain_id(self, chain_id: u64) -> Self {
        let state = self.state.clone();
        tokio::spawn(async move {
            state.data.write().await.chain_id = U256::from(chain_id);
        });
        self
    }

    pub fn with_accounts(self, accounts: Vec<Address>) -> Self {
        let state = self.state.clone();
        tokio::spawn(async move {
            state.data.write().await.accounts = accounts;
        });
        self
    }

    pub fn with_balance(self, address: Address, balance: U256) -> Self {
        let state = self.state.clone();
        tokio::spawn(async move {
            state.data.write().await.balances.insert(address, balance);
        });
        self
    }

    pub fn with_syncing(self, syncing: bool) -> Self {
        let state = self.state.clone();
        tokio::spawn(async move {
            state.data.write().await.syncing = syncing;
        });
        self
    }

    pub async fn start(
        self,
        addr: SocketAddr,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut module = RpcModule::new(());

        // Register RPC methods (both Ethereum and GolemBase)
        let rpc_impl = GolemBaseMock::new(self.state.clone());
        module.merge(EthRpcServer::into_rpc(rpc_impl.clone()))?;
        module.merge(GolemBaseRpcServer::into_rpc(rpc_impl))?;

        let server = Server::builder().build(addr).await?;

        let addr = server.local_addr()?;
        log::info!("GolemBase Mock Server listening on {}", addr);

        let server_handle = server.start(module);

        Ok(Self {
            state: self.state,
            server: Some(server_handle),
        })
    }

    pub fn state(&self) -> &MockState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut MockState {
        &mut self.state
    }
}

impl Default for GolemBaseMockServer {
    fn default() -> Self {
        Self::new()
    }
}
