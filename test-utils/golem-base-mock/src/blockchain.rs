use alloy::primitives::{Address, B256, U256};
use alloy::rlp::Decodable;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use golem_base_sdk::account::GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS;
use golem_base_sdk::entity::GolemBaseTransaction;
use golem_base_sdk::utils::wei_to_eth;

use crate::block::{Block, Transaction};
use crate::entity_db::{Entity, EntityDb};

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
    entity_db: Arc<EntityDb>,
}

impl Blockchain {
    /// Create a new empty mock blockchain
    pub fn new(entity_db: Arc<EntityDb>) -> Self {
        Self {
            state: Arc::new(RwLock::new(BlockchainState::default())),
            entity_db,
        }
    }

    /// Add a block to the blockchain
    pub async fn add_block(&self, block: Block) {
        let mut state = self.state.write().await;
        let block_number = block.header.block_number;
        let block_hash = block.header.block_hash;

        let block = Arc::new(block);
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

            // Extract and add entities from transaction data
            self.extract_entity_from_transaction(transaction, &block)
                .await;
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

        if transaction.value > U256::ZERO {
            log::debug!(
                "Account transfer: {} -> {} (value: {} ETH, tx: 0x{:x})",
                transaction.from,
                transaction.to,
                wei_to_eth(transaction.value),
                transaction.hash
            );
        }
    }

    /// Extract entity from transaction data and modify entity database state
    /// Checks if transaction is to storage contract and decodes GolemBase entity operations
    async fn extract_entity_from_transaction(&self, transaction: &Arc<Transaction>, block: &Block) {
        // Check if transaction is to the GolemBase storage processor contract
        if transaction.to != GOLEM_BASE_STORAGE_PROCESSOR_ADDRESS {
            return;
        }

        // Try to decode the transaction data as a GolemBaseTransaction
        // This is the inverse of the encoding shown in send_db_transaction
        if let Ok(golem_tx) = GolemBaseTransaction::decode(&mut transaction.data.as_ref()) {
            // Process creates
            for (idx, create) in golem_tx.creates.into_iter().enumerate() {
                let entity = Entity::create(create, transaction.from).with_hash(
                    block.header.block_number,
                    idx,
                    transaction.hash,
                );
                self.entity_db.add_entity(entity.clone()).await;
                log::info!(
                    "Entity created: 0x{:x}, owner: 0x{:x}",
                    entity.key,
                    entity.owner
                );
            }

            // Process updates
            for update in &golem_tx.updates {
                // Update the entity directly in the database
                self.entity_db
                    .update_entity(&update.entity_key, update)
                    .await;
                log::info!("Entity updated: 0x{:x}", update.entity_key);
            }

            // Process extensions
            for extend in &golem_tx.extensions {
                // For extensions, we need to update the existing entity's BTL
                self.entity_db
                    .update_entity_btl(&extend.entity_key, extend.number_of_blocks)
                    .await;
                log::info!(
                    "Entity extended: 0x{:x}, new BTL: {}",
                    extend.entity_key,
                    extend.number_of_blocks
                );
            }

            // Process deletes
            for delete in &golem_tx.deletes {
                if let Some(entity) = self.entity_db.remove_entity(delete).await {
                    log::info!(
                        "Entity deleted: 0x{:x}, owner: 0x{:x}",
                        entity.key,
                        entity.owner
                    );
                } else {
                    log::warn!("Entity not found for deletion: 0x{:x}", delete);
                }
            }
        }
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

    /// Find the block that contains a specific transaction
    pub async fn find_block_containing_transaction(
        &self,
        transaction_hash: &B256,
    ) -> Option<Arc<Block>> {
        let state = self.state.read().await;
        for (_, block) in &state.blocks_by_number {
            for tx in &block.transactions {
                if tx.hash == *transaction_hash {
                    return Some(block.clone());
                }
            }
        }
        None
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

    /// Get balance for an account
    pub async fn get_balance(&self, address: &Address) -> U256 {
        self.state
            .read()
            .await
            .accounts
            .get(address)
            .map(|account| account.balance)
            .unwrap_or(U256::ZERO)
    }

    /// Get all accounts
    pub async fn get_accounts(&self) -> Vec<Address> {
        self.state.read().await.accounts.keys().cloned().collect()
    }

    /// Get all blocks
    pub async fn get_blocks(&self) -> HashMap<u64, Arc<Block>> {
        self.state.read().await.blocks_by_number.clone()
    }

    /// Get the latest block number
    pub async fn get_latest_block_number(&self) -> anyhow::Result<u64> {
        let latest = self
            .state
            .read()
            .await
            .blocks_by_number
            .keys()
            .max()
            .copied();

        latest.ok_or_else(|| anyhow::anyhow!("No blocks found in blockchain"))
    }

    /// Add accounts with initial balances
    pub async fn add_accounts(&self, accounts: Vec<Address>) {
        let mut state = self.state.write().await;
        for address in accounts {
            state
                .accounts
                .entry(address)
                .or_insert_with(|| Account::new(address));
        }
    }

    /// Set balance for an account
    pub async fn set_balance(&self, address: Address, balance: U256) {
        let mut state = self.state.write().await;
        let account = state
            .accounts
            .entry(address)
            .or_insert_with(|| Account::new(address));
        account.balance = balance;
    }

    /// Create and add genesis block
    pub async fn create_genesis_block(&self) {
        let genesis_block = Block::new(
            0,                      // Genesis block number
            B256::ZERO,             // No previous block
            Vec::new(),             // No transactions
            U256::from(30_000_000), // Gas limit
            U256::ZERO,             // No gas used
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );

        self.add_block(genesis_block).await;
    }
}
