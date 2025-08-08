use alloy::primitives::{Address, B256, U256};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Represents an account in the mock blockchain
#[derive(Clone, Debug)]
pub struct Account {
    pub address: Address,
    pub nonce: U256,
    pub balance: U256,
    pub transactions: Vec<B256>,
    pub received_transactions: Vec<B256>,
}

impl Account {
    /// Create a new empty account
    pub fn new(address: Address) -> Self {
        Self {
            address,
            nonce: U256::ZERO,
            balance: U256::ZERO,
            transactions: Vec::new(),
            received_transactions: Vec::new(),
        }
    }

    /// Create a new empty account (alias for new)
    pub fn empty(address: Address) -> Self {
        Self::new(address)
    }
}

/// Represents a transaction in the mock blockchain
#[derive(Clone, Debug)]
pub struct Transaction {
    pub hash: B256,
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub gas: U256,
    pub gas_price: U256,
    pub nonce: U256,
    pub data: Bytes,
}

/// Represents the header of a block
#[derive(Clone, Debug)]
pub struct BlockHeader {
    pub block_number: u64,
    pub previous_block_hash: B256,
    pub block_hash: B256,
    pub gas_limit: U256,
    pub gas_used: U256,
    pub timestamp: u64,
}

/// Represents a block in the mock blockchain
#[derive(Clone, Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Arc<Transaction>>,
}

/// Internal state of the blockchain
#[derive(Clone, Debug, Default)]
struct BlockchainState {
    blocks_by_number: HashMap<u64, Arc<Block>>,
    blocks_by_hash: HashMap<B256, Arc<Block>>,
    transactions: HashMap<B256, Arc<Transaction>>,
    accounts: HashMap<Address, Account>,
}

/// Main mock blockchain structure
#[derive(Clone, Debug, Default)]
pub struct Blockchain {
    state: Arc<RwLock<BlockchainState>>,
}

impl Blockchain {
    /// Create a new empty mock blockchain
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(BlockchainState::default())),
        }
    }

    /// Add a block to the blockchain
    pub async fn add_block(&self, block: Arc<Block>) {
        let mut state = self.state.write().await;
        let block_number = block.header.block_number;
        let block_hash = block.header.block_hash;

        // Add block to block maps
        state.blocks_by_number.insert(block_number, block.clone());
        state.blocks_by_hash.insert(block_hash, block.clone());

        // Process all transactions in the block
        for transaction in &block.transactions {
            let transaction_hash = transaction.hash;

            // Add transaction to transactions map
            state
                .transactions
                .insert(transaction_hash, transaction.clone());

            // Update accounts
            Self::update_account_for_transaction(&mut state, &transaction);
        }
    }

    /// Update account state based on a transaction
    fn update_account_for_transaction(state: &mut BlockchainState, transaction: &Arc<Transaction>) {
        // Update sender account
        let sender = transaction.from;
        let sender_account = state
            .accounts
            .entry(sender)
            .or_insert_with(|| Account::new(sender));
        sender_account.nonce += U256::from(1);
        sender_account.balance = sender_account.balance.saturating_sub(transaction.value);
        sender_account.transactions.push(transaction.hash);

        // Update receiver account
        let receiver = transaction.to;
        let receiver_account = state
            .accounts
            .entry(receiver)
            .or_insert_with(|| Account::new(receiver));
        receiver_account.balance += transaction.value;
        receiver_account
            .received_transactions
            .push(transaction.hash);
    }

    /// Get a block by its number
    pub async fn get_block_by_number(&self, block_number: u64) -> Option<Arc<Block>> {
        self.state
            .read()
            .await
            .blocks_by_number
            .get(&block_number)
            .cloned()
    }

    /// Get a block by its hash
    pub async fn get_block_by_hash(&self, block_hash: &B256) -> Option<Arc<Block>> {
        self.state
            .read()
            .await
            .blocks_by_hash
            .get(block_hash)
            .cloned()
    }

    /// Get a transaction by its hash
    pub async fn get_transaction(&self, transaction_hash: &B256) -> Option<Arc<Transaction>> {
        self.state
            .read()
            .await
            .transactions
            .get(transaction_hash)
            .cloned()
    }

    /// Get an account by its address
    pub async fn get_account(&self, address: &Address) -> Option<Account> {
        self.state.read().await.accounts.get(address).cloned()
    }

    /// Get a mutable reference to an account by its address
    pub async fn get_account_mut(&self, address: &Address) -> Option<Account> {
        // For now, return a clone since we can't return a mutable reference from async
        self.state.read().await.accounts.get(address).cloned()
    }
}
