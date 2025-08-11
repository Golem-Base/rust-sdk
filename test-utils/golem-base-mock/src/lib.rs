use alloy::consensus::{EthereumTxEnvelope, TxEip4844, TxEip4844Variant};
use alloy::primitives::{Address, Bytes, B256, U256};
use alloy::rlp::Decodable;
use alloy::rpc::types::{Block, BlockId, BlockNumberOrTag, Transaction, TransactionReceipt};
use anyhow::Result;
use jsonrpsee::core::{async_trait, RpcResult};
use jsonrpsee::server::{RpcModule, Server};
use jsonrpsee::types::{ErrorCode, ErrorObject};
use rand::Rng;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Helper function to create ErrorObject with a typed ErrorCode and message
fn create_error(code: ErrorCode, message: impl Into<String>) -> ErrorObject<'static> {
    ErrorObject::owned(code.code(), message.into(), None::<()>)
}

use crate::api::{EthRpcServer, GolemBaseRpcServer};
use crate::blockchain::Blockchain;
use crate::entity_db::EntityDb;
use crate::execution::ExecutionEngine;
use crate::transaction_pool::TransactionPool;

pub mod api;
pub mod block;
pub mod blockchain;
pub mod entity_db;
pub mod execution;
pub mod server;
pub mod transaction_pool;

// Re-export server functions for convenience
pub use server::{create_test_mock_server, get_default_mock_server_url};

/// Mock implementation of RPC methods (both Ethereum and GolemBase)
#[derive(Clone, Default)]
pub struct GolemBaseMock {
    chain_id: U256,
    blockchain: Blockchain,
    entity_db: EntityDb,
    transaction_pool: TransactionPool,
    execution: ExecutionEngine,
}

impl GolemBaseMock {
    pub fn new() -> Self {
        let entity_db = EntityDb::new();
        let blockchain = Blockchain::new(Arc::new(entity_db.clone()));
        let transaction_pool = TransactionPool::new();
        let execution_engine = ExecutionEngine::new(
            Arc::new(RwLock::new(blockchain.clone())),
            Arc::new(transaction_pool.clone()),
        );

        Self {
            chain_id: U256::from(1337),
            blockchain,
            entity_db,
            transaction_pool,
            execution: execution_engine,
        }
    }
}

#[async_trait]
impl EthRpcServer for GolemBaseMock {
    async fn get_transaction_count(
        &self,
        address: Address,
        _block: Option<BlockId>,
    ) -> RpcResult<U256> {
        let count = self.transaction_pool.get_transaction_count(&address).await;
        Ok(count)
    }

    async fn get_transaction_receipt(&self, _hash: B256) -> RpcResult<Option<TransactionReceipt>> {
        // Mock implementation - return None for now
        // In a real implementation, this would return actual transaction receipts
        Ok(None)
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
        Ok(self.blockchain.get_balance(&address).await)
    }

    async fn accounts(&self) -> RpcResult<Vec<Address>> {
        Ok(self.blockchain.get_accounts().await)
    }

    async fn get_accounts(&self) -> RpcResult<Vec<Address>> {
        Ok(self.blockchain.get_accounts().await)
    }

    async fn send_transaction(&self, _transaction: serde_json::Value) -> RpcResult<B256> {
        // Mock implementation - return a random transaction hash
        let mut rng = rand::thread_rng();
        let tx_hash = B256::new(rng.gen());
        Ok(tx_hash)
    }

    async fn send_raw_transaction(&self, data: Bytes) -> RpcResult<B256> {
        // Use the bytes directly since input is already Bytes
        let tx_bytes = data.to_vec();

        // Decode the RLP-encoded transaction
        let decoded = EthereumTxEnvelope::<TxEip4844>::decode(&mut &tx_bytes[..]).map_err(|e| {
            create_error(
                ErrorCode::ParseError,
                format!("Failed to decode transaction: {e}"),
            )
        })?;

        // Convert decoded transaction to our internal Transaction type
        let transaction = crate::block::Transaction::try_from(decoded).map_err(|e| {
            create_error(
                ErrorCode::InvalidParams,
                format!("Failed to convert transaction: {e}"),
            )
        })?;

        let transaction = Arc::new(transaction);
        self.transaction_pool
            .add_transaction(transaction.clone())
            .await;

        Ok(transaction.hash)
    }

    async fn chain_id(&self) -> RpcResult<U256> {
        Ok(self.chain_id)
    }

    async fn get_transaction_by_hash(&self, hash: B256) -> RpcResult<Option<Transaction>> {
        // First check the blockchain for mined transactions (to get block info)
        if let Some(tx) = self.blockchain.get_transaction(&hash).await {
            // Since block is added to chain, we need to find in which exact block it is.
            let block = self
                .blockchain
                .find_block_containing_transaction(&hash)
                .await
                .ok_or(create_error(
                    ErrorCode::InternalError,
                    format!("Failed to find block containing transaction: {hash}"),
                ))?;
            let idx = block.find_transaction_index(&hash).ok_or(create_error(
                ErrorCode::InternalError,
                format!("Failed to find transaction index in block: {hash}"),
            ))?;

            // Build Transaction from block and transaction data
            let tx_envelope: EthereumTxEnvelope<TxEip4844Variant> = tx.to_envelope();
            let alloy_tx = Transaction {
                inner: tx_envelope.try_into_recovered().map_err(|e| {
                    create_error(
                        ErrorCode::InternalError,
                        format!("Failed to convert transaction: {e}"),
                    )
                })?,
                block_hash: Some(block.header.block_hash),
                block_number: Some(block.header.block_number),
                transaction_index: Some(idx),
                effective_gas_price: None,
            };
            return Ok(Some(alloy_tx));
        }

        // If not found in blockchain, check the transaction pool for pending transactions
        if let Some(tx) = self.transaction_pool.get_transaction(&hash).await {
            // Convert our internal Transaction to alloy Transaction format for pending transactions
            let tx_envelope: EthereumTxEnvelope<TxEip4844Variant> = tx.to_envelope();
            let alloy_tx = Transaction {
                inner: tx_envelope.try_into_recovered().map_err(|e| {
                    create_error(
                        ErrorCode::InternalError,
                        format!("Failed to convert pending transaction: {e}"),
                    )
                })?,
                block_hash: None,
                block_number: None,
                transaction_index: None,
                effective_gas_price: None,
            };
            return Ok(Some(alloy_tx));
        }

        // Transaction not found in pool or blockchain
        Ok(None)
    }

    async fn syncing(&self) -> RpcResult<bool> {
        Ok(false) // Mock implementation - always false
    }

    async fn get_block_by_number(
        &self,
        block: BlockNumberOrTag,
        _full: Option<bool>,
    ) -> RpcResult<Option<Block>> {
        // Get block number from BlockNumberOrTag
        let block_number = match block {
            BlockNumberOrTag::Number(num) => num,
            BlockNumberOrTag::Latest | BlockNumberOrTag::Safe | BlockNumberOrTag::Finalized => {
                // For mock, just return the latest block number
                match self.blockchain.get_latest_block_number().await {
                    Ok(num) => num,
                    Err(_) => return Ok(None), // Return None if no blocks exist
                }
            }
            BlockNumberOrTag::Earliest => 0,
            BlockNumberOrTag::Pending => {
                // Return None for pending blocks in mock implementation
                return Ok(None);
            }
        };

        let block = self.blockchain.get_block_by_number(block_number).await;
        Ok(block.map(|block| (*block).clone().into()))
    }

    async fn estimate_gas(&self, _call_request: serde_json::Value) -> RpcResult<U256> {
        // Mock implementation - return a reasonable gas estimate
        // In a real implementation, this would simulate the transaction and estimate gas
        Ok(U256::from(21000))
    }

    async fn fee_history(
        &self,
        _block_count: U256,
        _newest_block: BlockId,
        _reward_percentiles: Option<Vec<f64>>,
    ) -> RpcResult<serde_json::Value> {
        // Mock implementation - return empty fee history
        // In a real implementation, this would return actual fee history data
        Ok(serde_json::json!({
            "oldestBlock": "0x0",
            "baseFeePerGas": [],
            "gasUsedRatio": [],
            "reward": []
        }))
    }

    async fn gas_price(&self) -> RpcResult<U256> {
        // Mock implementation - return a reasonable gas price
        // In a real implementation, this would return the current gas price from the network
        Ok(U256::from(20_000_000_000u64)) // 20 gwei
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
#[derive(Clone, Default)]
pub struct GolemBaseMockServer {
    pub state: GolemBaseMock,
    server: Option<jsonrpsee::server::ServerHandle>,
}

impl GolemBaseMockServer {
    pub fn new() -> Self {
        Self {
            state: GolemBaseMock::new(),
            server: None,
        }
    }

    pub fn with_chain_id(mut self, chain_id: u64) -> Self {
        self.state.chain_id = U256::from(chain_id);
        self
    }

    pub fn with_accounts(self, accounts: Vec<Address>) -> Self {
        let state = self.state.clone();
        tokio::spawn(async move {
            state.blockchain.add_accounts(accounts).await;
        });
        self
    }

    pub fn with_balance(self, address: Address, balance: U256) -> Self {
        let state = self.state.clone();
        tokio::spawn(async move {
            state.blockchain.set_balance(address, balance).await;
        });
        self
    }

    pub async fn start(
        self,
        addr: SocketAddr,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut module = RpcModule::new(());

        // Register RPC methods (both Ethereum and GolemBase)
        let rpc_impl = self.state.clone();
        module.merge(EthRpcServer::into_rpc(rpc_impl.clone()))?;
        module.merge(GolemBaseRpcServer::into_rpc(rpc_impl))?;

        let server = Server::builder().build(addr).await?;

        let addr = server.local_addr()?;
        log::info!("GolemBase Mock Server listening on {}", addr);

        // Start the execution engine to produce blocks
        self.state.blockchain.create_genesis_block().await;
        self.state.execution.start().await;

        let server_handle = server.start(module);

        Ok(Self {
            state: self.state,
            server: Some(server_handle),
        })
    }
}
