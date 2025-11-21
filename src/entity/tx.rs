use alloy::{rpc::types::TransactionReceipt, sol_types::SolEventInterface};
use alloy_rlp::{Encodable, RlpDecodable, RlpEncodable};
use bon::bon;

use crate::eth::{self, GolemBaseABI};

/// Type representing a transaction in GolemBase, including creates, updates, deletes, and extensions.
/// Used as the main payload for submitting entity changes to the chain.
#[derive(Debug, Clone)]
pub struct GolemBaseTransaction {
    pub encodable: EncodableGolemBaseTransaction,
    pub gas_limit: Option<u64>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
}

// A transaction that can be encoded in RLP
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable)]
pub struct EncodableGolemBaseTransaction {
    /// A list of entities to create.
    pub creates: Vec<Create>,
    /// A list of entities to update.
    pub updates: Vec<Update>,
    /// A list of entity keys to delete.
    pub deletes: Vec<GolemBaseDelete>,
    /// A list of entities to extend.
    pub extensions: Vec<Extend>,
}

#[derive(Debug, Default)]
pub struct TransactionResult {
    pub creates: Vec<EntityResult>,
    pub updates: Vec<EntityResult>,
    pub deletes: Vec<DeleteResult>,
    pub extensions: Vec<ExtendResult>,
}

impl TryFrom<TransactionReceipt> for TransactionResult {
    type Error = eth::Error;

    fn try_from(receipt: TransactionReceipt) -> Result<Self, Self::Error> {
        if !receipt.status() {
            return Err(Self::Error::TransactionReceiptError(format!(
                "Transaction {} failed: {:?}",
                receipt.transaction_hash, receipt
            )));
        }

        let mut txres = TransactionResult::default();
        receipt.logs().iter().cloned().try_for_each(|log| {
            let log: alloy::primitives::Log = log.into();
            let parsed = GolemBaseABI::GolemBaseABIEvents::decode_log(&log).map_err(|e| {
                Self::Error::UnexpectedLogDataError(format!("Error decoding event log: {e}"))
            })?;
            match parsed.data {
                GolemBaseABI::GolemBaseABIEvents::GolemBaseStorageEntityCreated(data) => {
                    txres.creates.push(EntityResult {
                        entity_key: data.entityKey.into(),
                        expiration_block: data.expirationBlock.try_into().unwrap_or_default(),
                    });
                    Ok(())
                }
                GolemBaseABI::GolemBaseABIEvents::GolemBaseStorageEntityUpdated(data) => {
                    txres.updates.push(EntityResult {
                        entity_key: data.entityKey.into(),
                        expiration_block: data.expirationBlock.try_into().unwrap_or_default(),
                    });
                    Ok(())
                }
                GolemBaseABI::GolemBaseABIEvents::GolemBaseStorageEntityDeleted(data) => {
                    txres.deletes.push(DeleteResult {
                        entity_key: data.entityKey.into(),
                    });
                    Ok(())
                }
                GolemBaseABI::GolemBaseABIEvents::GolemBaseStorageEntityBTLExtended(data) => {
                    txres.extensions.push(ExtendResult {
                        entity_key: data.entityKey.into(),
                        old_expiration_block: data
                            .oldExpirationBlock
                            .try_into()
                            .unwrap_or_default(),
                        new_expiration_block: data
                            .newExpirationBlock
                            .try_into()
                            .unwrap_or_default(),
                    });
                    Ok(())
                }
            }
        })?;

        Ok(txres)
    }
}

#[bon]
impl GolemBaseTransaction {
    #[builder]
    pub fn builder(
        creates: Option<Vec<Create>>,
        updates: Option<Vec<Update>>,
        deletes: Option<Vec<GolemBaseDelete>>,
        extensions: Option<Vec<Extend>>,
        gas_limit: Option<u64>,
        max_priority_fee_per_gas: Option<u128>,
        max_fee_per_gas: Option<u128>,
    ) -> Self {
        Self {
            encodable: EncodableGolemBaseTransaction {
                creates: creates.unwrap_or_default(),
                updates: updates.unwrap_or_default(),
                deletes: deletes.unwrap_or_default(),
                extensions: extensions.unwrap_or_default(),
            },
            gas_limit,
            max_priority_fee_per_gas,
            max_fee_per_gas,
        }
    }
}

impl GolemBaseTransaction {
    /// Returns the RLP-encoded bytes of the transaction.
    /// Useful for submitting the transaction to the chain.
    pub fn encoded(&self) -> Vec<u8> {
        let mut encoded = Vec::new();
        self.encodable.encode(&mut encoded);
        encoded
    }
}
