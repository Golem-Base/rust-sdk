use crate::blockchain::{Block, BlockHeader, Blockchain};
use crate::entity_db::EntityDb;
use crate::transaction_pool::TransactionPool;
use alloy::primitives::{B256, U256};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

/// Internal state of the execution engine
#[derive(Clone, Debug)]
struct ExecutionEngineState {
    current_block_number: u64,
    running: bool,
}

/// Execution engine that mines blocks and processes transactions
#[derive(Clone, Debug)]
pub struct ExecutionEngine {
    blockchain: Arc<RwLock<Blockchain>>,
    entity_db: Arc<EntityDb>,
    transaction_pool: Arc<TransactionPool>,
    state: Arc<RwLock<ExecutionEngineState>>,
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub fn new(
        blockchain: Arc<RwLock<Blockchain>>,
        entity_db: Arc<EntityDb>,
        transaction_pool: Arc<TransactionPool>,
    ) -> Self {
        Self {
            blockchain,
            entity_db,
            transaction_pool,
            state: Arc::new(RwLock::new(ExecutionEngineState {
                current_block_number: 0,
                running: false,
            })),
        }
    }

    /// Start the execution engine
    pub async fn start(&self) {
        let mut state = self.state.write().await;
        if state.running {
            return; // Already running
        }
        state.running = true;
        drop(state);

        let engine = self.clone();
        tokio::spawn(async move {
            engine.run().await;
        });
    }

    /// Stop the execution engine
    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        state.running = false;
    }

    /// Main execution loop
    async fn run(&self) {
        let mut interval = interval(Duration::from_secs(1));

        while self.state.read().await.running {
            interval.tick().await;
            self.mine_block().await;
        }
    }

    /// Mine a new block
    async fn mine_block(&self) {
        // Get current block number
        let block_number = {
            let mut state = self.state.write().await;
            state.current_block_number += 1;
            state.current_block_number
        };

        // Get transactions from pool (limit to 100 per block)
        let transactions = self.transaction_pool.get_transaction_batch(100).await;

        if transactions.is_empty() {
            // Create empty block if no transactions
            self.create_empty_block(block_number).await;
        } else {
            // Process transactions and create block
            self.process_transactions_and_create_block(block_number, transactions)
                .await;
        }
    }

    /// Create an empty block
    async fn create_empty_block(&self, block_number: u64) {
        let block = self.create_block(block_number, vec![]);

        // Add block to blockchain
        let mut blockchain = self.blockchain.write().await;
        blockchain.add_block(Arc::new(block));
    }

    /// Process transactions and create a block
    async fn process_transactions_and_create_block(
        &self,
        block_number: u64,
        transactions: Vec<Arc<crate::blockchain::Transaction>>,
    ) {
        let mut entities_to_add = Vec::new();

        // Process each transaction
        for transaction in &transactions {
            // Extract entities from transaction data (simplified)
            // In a real implementation, you would parse the transaction data
            // and extract GolemBase entity operations
            if let Some(entity) = self.extract_entity_from_transaction(transaction).await {
                entities_to_add.push(entity);
            }
        }

        // Add entities to the database
        for entity in entities_to_add {
            self.entity_db.add_entity(entity).await;
        }

        // Remove processed transactions from pool
        for transaction in &transactions {
            self.transaction_pool
                .remove_transaction(&transaction.hash)
                .await;
        }

        // Create and add block
        let block = self.create_block(block_number, transactions);
        let mut blockchain = self.blockchain.write().await;
        blockchain.add_block(Arc::new(block));
    }

    /// Create a block with the given transactions
    fn create_block(
        &self,
        block_number: u64,
        transactions: Vec<Arc<crate::blockchain::Transaction>>,
    ) -> Block {
        // Generate a mock block hash (in real implementation, this would be computed)
        let block_hash = B256::from_slice(&[block_number as u8; 32]);
        let previous_block_hash = if block_number == 1 {
            B256::ZERO
        } else {
            // In a real implementation, you'd get the previous block's hash
            B256::from_slice(&[(block_number - 1) as u8; 32])
        };

        let header = BlockHeader {
            block_number,
            previous_block_hash,
            block_hash,
            gas_limit: U256::from(30_000_000),
            gas_used: U256::from(transactions.len() * 21_000), // Simple gas calculation
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        Block {
            header,
            transactions,
        }
    }

    /// Extract entity from transaction (simplified implementation)
    async fn extract_entity_from_transaction(
        &self,
        _transaction: &Arc<crate::blockchain::Transaction>,
    ) -> Option<crate::entity_db::Entity> {
        // This is a simplified implementation
        // In a real system, you would parse the transaction data
        // and extract GolemBase entity operations

        // For now, return None (no entities extracted)
        None
    }

    /// Get current block number
    pub async fn get_current_block_number(&self) -> u64 {
        self.state.read().await.current_block_number
    }

    /// Check if the engine is running
    pub async fn is_running(&self) -> bool {
        self.state.read().await.running
    }
}
