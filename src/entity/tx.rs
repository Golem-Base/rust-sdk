use alloy::{rpc::types::TransactionReceipt, sol_types::SolEventInterface};
use alloy_rlp::{Encodable, RlpDecodable, RlpEncodable};
use bon::bon;

use crate::{
    entity::{
        EntityResult,
        create::Create,
        delete::{Delete, DeleteResult},
        extend::{Extend, ExtendResult},
        update::Update,
    },
    eth::{self, ArkivAbi},
};

/// Type representing a transaction in GolemBase, including creates, updates, deletes, and extensions.
/// Used as the main payload for submitting entity changes to the chain.
#[derive(Debug, Clone)]
pub struct Transaction {
    pub encodable: EncodableTransaction,
    pub gas_limit: Option<u64>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
}

// A transaction that can be encoded in RLP
#[derive(Debug, Clone, Default, RlpEncodable, RlpDecodable)]
pub struct EncodableTransaction {
    // TODO: Add chown
    /// A list of [`Create`] operations.
    pub creates: Vec<Create>,
    /// A list of [`Update`] operations.
    pub updates: Vec<Update>,
    /// A list of [`Delete`] operations.
    pub deletes: Vec<Delete>,
    /// A list of [`Extend`] operations.
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
            let parsed = ArkivAbi::ArkivAbiEvents::decode_log(&log).map_err(|e| {
                Self::Error::UnexpectedLogDataError(format!("Error decoding event log: {e}"))
            })?;
            match parsed.data {
                ArkivAbi::ArkivAbiEvents::EntityCreated(data) => {
                    txres.creates.push(EntityResult {
                        entity_key: data.entityKey.into(),
                        expiration_block: data.expirationBlock.try_into().unwrap_or_default(),
                    });
                    Ok(())
                }
                ArkivAbi::ArkivAbiEvents::EntityUpdated(data) => {
                    txres.updates.push(EntityResult {
                        entity_key: data.entityKey.into(),
                        expiration_block: data.expirationBlock.try_into().unwrap_or_default(),
                    });
                    Ok(())
                }
                ArkivAbi::ArkivAbiEvents::EntityDeleted(data) => {
                    txres.deletes.push(DeleteResult {
                        entity_key: data.entityKey.into(),
                    });
                    Ok(())
                }
                ArkivAbi::ArkivAbiEvents::EntityExtended(data) => {
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
impl Transaction {
    #[builder]
    pub fn builder(
        creates: Option<Vec<Create>>,
        updates: Option<Vec<Update>>,
        deletes: Option<Vec<Delete>>,
        extensions: Option<Vec<Extend>>,
        gas_limit: Option<u64>,
        max_priority_fee_per_gas: Option<u128>,
        max_fee_per_gas: Option<u128>,
    ) -> Self {
        Self {
            encodable: EncodableTransaction {
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

impl Transaction {
    /// Returns the RLP-encoded bytes of the transaction.
    /// Useful for submitting the transaction to the chain.
    pub fn encoded(&self) -> Vec<u8> {
        let mut encoded = Vec::new();
        self.encodable.encode(&mut encoded);
        encoded
    }
}
