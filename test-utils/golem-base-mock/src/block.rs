use alloy::consensus::{EthereumTxEnvelope, Signed, TxEip4844, TxEip4844Variant};
use alloy::consensus::{Header as ConsensusHeader, Transaction as _};
use alloy::primitives::{Address, Bloom, Bytes, Signature, B256, B64, U256};
use alloy::rpc::types::{
    AccessList, Block as AlloyBlock, BlockTransactions, Header as AlloyHeader,
};
use anyhow::anyhow;
use std::sync::Arc;

/// Represents a transaction in the mock blockchain
#[derive(Clone, Debug)]
pub struct Transaction {
    pub hash: B256,
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub gas_limit: u64,
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: u128,
    pub max_fee_per_blob_gas: u128,
    pub nonce: u64,
    pub data: Bytes,
    pub chain_id: u64,
    pub signature: Signature,
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

    /// Find the index of a specific transaction in this block
    pub fn find_transaction_index(&self, transaction_hash: &B256) -> Option<u64> {
        for (index, tx) in self.transactions.iter().enumerate() {
            if tx.hash == *transaction_hash {
                return Some(index as u64);
            }
        }
        None
    }
}

impl Transaction {
    pub fn to_envelope(self: &Self) -> EthereumTxEnvelope<TxEip4844Variant> {
        self.clone().into()
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

impl TryFrom<EthereumTxEnvelope<TxEip4844>> for Transaction {
    type Error = anyhow::Error;

    fn try_from(decoded: EthereumTxEnvelope<TxEip4844>) -> Result<Self, Self::Error> {
        // Check if this is an EIP-4844 transaction
        if !matches!(decoded, EthereumTxEnvelope::Eip4844(_)) {
            return Err(anyhow!("Unsupported transaction type"));
        }

        let transaction = Self {
            hash: decoded.tx_hash().clone(),
            from: decoded
                .recover_signer()
                .map_err(|e| anyhow!("Failed to recover signer: {e}"))?,
            to: decoded.to().unwrap_or(Address::ZERO),
            value: decoded.value(),
            gas_limit: decoded.gas_limit(),
            max_fee_per_gas: decoded.max_fee_per_gas(),
            max_priority_fee_per_gas: decoded
                .max_priority_fee_per_gas()
                .ok_or(anyhow!("Missing max priority fee per gas"))?,
            max_fee_per_blob_gas: decoded
                .max_fee_per_blob_gas()
                .ok_or(anyhow!("Missing max fee per blob gas"))?,
            nonce: decoded.nonce(),
            data: decoded.input().clone(),
            chain_id: decoded.chain_id().ok_or(anyhow!("No chain id"))?,
            signature: decoded.signature().clone(),
        };

        Ok(transaction)
    }
}

impl Into<EthereumTxEnvelope<TxEip4844Variant>> for Transaction {
    fn into(self) -> EthereumTxEnvelope<TxEip4844Variant> {
        // Create an EIP-4844 transaction envelope
        EthereumTxEnvelope::Eip4844(Signed::new_unchecked(
            TxEip4844Variant::TxEip4844(TxEip4844 {
                chain_id: self.chain_id,
                nonce: self.nonce,
                max_fee_per_gas: self.max_fee_per_gas,
                max_priority_fee_per_gas: self.max_priority_fee_per_gas,
                gas_limit: self.gas_limit,
                to: self.to,
                value: self.value,
                input: self.data,
                access_list: AccessList::default(),
                blob_versioned_hashes: vec![],
                max_fee_per_blob_gas: self.max_fee_per_blob_gas,
            }),
            self.signature,
            self.hash,
        ))
    }
}
