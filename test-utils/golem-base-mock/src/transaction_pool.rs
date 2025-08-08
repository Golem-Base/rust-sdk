use crate::blockchain::Transaction;
use alloy::primitives::B256;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Transaction pool that holds pending transactions
#[derive(Clone, Debug, Default)]
pub struct TransactionPool {
    state: Arc<RwLock<HashMap<B256, Arc<Transaction>>>>,
}

impl TransactionPool {
    /// Create a new empty transaction pool
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a transaction to the pool
    pub async fn add_transaction(&self, transaction: Arc<Transaction>) {
        let hash = transaction.hash;
        self.state.write().await.insert(hash, transaction);
    }

    /// Get a transaction from the pool by hash
    pub async fn get_transaction(&self, hash: &B256) -> Option<Arc<Transaction>> {
        self.state.read().await.get(hash).cloned()
    }

    /// Remove a transaction from the pool (when it gets mined)
    pub async fn remove_transaction(&self, hash: &B256) -> Option<Arc<Transaction>> {
        self.state.write().await.remove(hash)
    }

    /// Get all pending transactions
    pub async fn get_all_transactions(&self) -> Vec<Arc<Transaction>> {
        self.state.read().await.values().cloned().collect()
    }

    /// Get a batch of transactions (for mining)
    pub async fn get_transaction_batch(&self, max_count: usize) -> Vec<Arc<Transaction>> {
        let transactions = self.state.read().await;
        transactions.values().take(max_count).cloned().collect()
    }

    /// Get the number of pending transactions
    pub async fn count(&self) -> usize {
        self.state.read().await.len()
    }

    /// Check if the pool is empty
    pub async fn is_empty(&self) -> bool {
        self.state.read().await.is_empty()
    }
}
