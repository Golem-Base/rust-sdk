use alloy::consensus::Header as ConsensusHeader;
use alloy::primitives::{Address, Bloom, Bytes, B256, B64, U256};
use alloy::rpc::types::{Block as AlloyBlock, BlockTransactions, Header as AlloyHeader};
use std::sync::Arc;

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

impl Block {
    /// Compute block hash based on block content and previous block hash
    pub fn compute_hash(
        block_number: u64,
        previous_block_hash: B256,
        transactions: &[Arc<Transaction>],
    ) -> B256 {
        let mut hash_bytes = [0u8; 32];

        // Use block number in first 8 bytes
        hash_bytes[0..8].copy_from_slice(&block_number.to_le_bytes());

        // Use previous block hash in next 32 bytes (wrapped around)
        let prev_hash_slice = previous_block_hash.as_slice();
        for i in 0..32 {
            hash_bytes[i] ^= prev_hash_slice[i];
        }

        // Use transaction count
        hash_bytes[8..16].copy_from_slice(&transactions.len().to_le_bytes());

        // Mix in transaction hashes
        for tx in transactions {
            let tx_hash_slice = tx.hash.as_slice();
            for j in 0..32 {
                hash_bytes[j] ^= tx_hash_slice[j];
            }
        }

        B256::from_slice(&hash_bytes)
    }

    /// Create a new block with computed hash
    pub fn new(
        block_number: u64,
        previous_block_hash: B256,
        transactions: Vec<Arc<Transaction>>,
        gas_limit: U256,
        gas_used: U256,
        timestamp: u64,
    ) -> Self {
        let block_hash = Self::compute_hash(block_number, previous_block_hash, &transactions);

        let header = BlockHeader {
            block_number,
            previous_block_hash,
            block_hash,
            gas_limit,
            gas_used,
            timestamp,
        };

        Self {
            header,
            transactions,
        }
    }
}

impl Into<AlloyBlock> for Block {
    fn into(self) -> AlloyBlock {
        AlloyBlock {
            header: AlloyHeader {
                hash: self.header.block_hash,
                inner: ConsensusHeader {
                    parent_hash: self.header.previous_block_hash,
                    ommers_hash: B256::ZERO,       // Mock: no ommers
                    beneficiary: Address::ZERO,    // Mock: no miner
                    state_root: B256::ZERO,        // Mock: empty state
                    transactions_root: B256::ZERO, // Mock: empty transactions root
                    receipts_root: B256::ZERO,     // Mock: empty receipts root
                    logs_bloom: Bloom::ZERO,       // Mock: empty bloom
                    difficulty: U256::ZERO,        // Mock: PoS block
                    number: self.header.block_number,
                    gas_limit: self.header.gas_limit.try_into().unwrap_or(u64::MAX),
                    gas_used: self.header.gas_used.try_into().unwrap_or(0),
                    timestamp: self.header.timestamp,
                    extra_data: Bytes::new(),       // Mock: no extra data
                    mix_hash: B256::ZERO,           // Mock: no mix hash
                    nonce: B64::ZERO,               // Mock: no nonce
                    base_fee_per_gas: None,         // Mock: no base fee
                    withdrawals_root: None,         // Mock: no withdrawals
                    blob_gas_used: None,            // Mock: no blob gas
                    excess_blob_gas: None,          // Mock: no excess blob gas
                    parent_beacon_block_root: None, // Mock: no beacon root
                    requests_hash: None,            // Mock: no requests
                },
                total_difficulty: Some(U256::ZERO),
                size: Some(U256::from(0)),
            },
            uncles: Vec::new(),
            transactions: BlockTransactions::Hashes(
                self.transactions.iter().map(|tx| tx.hash).collect(),
            ),
            withdrawals: None,
        }
    }
}
