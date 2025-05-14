use alloy::primitives::{keccak256, Address};
use alloy::providers::DynProvider;
use alloy::signers::k256::ecdsa::{SigningKey, VerifyingKey};
use alloy::signers::local::{LocalSignerError, PrivateKeySigner};
use alloy::signers::{Signature, SignerSync};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use rand::thread_rng;
use std::fs;
use std::path::PathBuf;

use crate::account::TransactionSigner;
use crate::Hash;

const DEFAULT_KEYSTORE_DIR: &str = "golembase";

/// A signer that keeps the private key in memory
pub struct InMemorySigner {
    signer: PrivateKeySigner,
}

impl InMemorySigner {
    /// Gets the default keystore directory path
    pub fn get_keystore_dir() -> anyhow::Result<PathBuf> {
        let path = dirs::config_dir()
            .context("Could not find home directory")?
            .join(DEFAULT_KEYSTORE_DIR);

        // Create directory only if it doesn't exist
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        Ok(path)
    }

    /// Generates a new random private key
    pub fn generate() -> Self {
        let signer = PrivateKeySigner::random();
        Self { signer }
    }

    /// Returns the private key
    pub fn private_key(&self) -> SigningKey {
        self.signer.credential().clone()
    }

    /// Returns the public key
    pub fn public_key(&self) -> VerifyingKey {
        *self.signer.credential().verifying_key()
    }

    /// Saves the private key to a file in the standard directory using keystore format
    pub fn save(&self, password: &str) -> anyhow::Result<PathBuf> {
        let path = Self::get_keystore_dir()?;
        let name = format!("key_{}.json", self.address());

        let mut rng = thread_rng();
        PrivateKeySigner::encrypt_keystore(
            &path,
            &mut rng,
            self.signer.credential().to_bytes(),
            password,
            Some(&name),
        )?;

        Ok(path)
    }

    /// Loads a private key from a keystore file
    pub fn load_keystore(path: PathBuf, password: &str) -> anyhow::Result<Self> {
        let signer = PrivateKeySigner::decrypt_keystore(&path, password).map_err(|e| match e {
            LocalSignerError::EcdsaError(e) => anyhow!("ECDSA error: {e}"),
            LocalSignerError::EthKeystoreError(e) => anyhow!("Keystore error: {e}"),
            e => anyhow!("Error loading key: {e}"),
        })?;
        Ok(Self { signer })
    }

    /// Loads a signer by address from the default directory
    pub fn load_by_address(address: Address, password: &str) -> anyhow::Result<Self> {
        let path = Self::get_keystore_dir()?.join(format!("key_{}.json", address));
        Self::load_keystore(path, password)
    }

    /// Loads a signer from a raw private key file
    pub fn load_raw_key(path: PathBuf) -> anyhow::Result<Self> {
        let private_key_bytes =
            fs::read(&path).map_err(|e| anyhow!("Failed to read private key file: {}", e))?;

        let private_key = Hash::from_slice(&private_key_bytes);
        let signer = PrivateKeySigner::from_bytes(&private_key)
            .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;

        Ok(Self { signer })
    }

    /// Lists all local accounts in the keystore directory
    pub fn list_local_accounts() -> anyhow::Result<Vec<Address>> {
        let keystore_dir = Self::get_keystore_dir()?;
        let mut accounts = Vec::new();

        if let Ok(entries) = std::fs::read_dir(keystore_dir) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if let Some(address) = Self::parse_keystore_filename(file_name) {
                        accounts.push(address);
                    }
                }
            }
        }

        Ok(accounts)
    }

    /// Parses an address from a keystore filename
    fn parse_keystore_filename(file_name: &str) -> Option<Address> {
        if !file_name.starts_with("key_") || !file_name.ends_with(".json") {
            return None;
        }

        file_name
            .strip_prefix("key_")
            .and_then(|s| s.strip_suffix(".json"))
            .and_then(|address_str| Address::parse_checksummed(address_str, None).ok())
    }
}

#[async_trait]
impl TransactionSigner for InMemorySigner {
    fn address(&self) -> Address {
        self.signer.address()
    }

    async fn sign(&self, data: &[u8]) -> anyhow::Result<Signature> {
        let hash = keccak256(data);
        Ok(self.signer.sign_hash_sync(&hash)?)
    }
}

/// A signer that uses GolemBase to sign transactions
#[allow(dead_code)]
pub struct GolemBaseSigner {
    /// The address of the account
    address: Address,
    /// The provider for signing
    provider: DynProvider,
    /// The chain ID for signing
    chain_id: u64,
}

impl GolemBaseSigner {
    /// Creates a new GolemBase signer
    pub fn new(address: Address, provider: DynProvider, chain_id: u64) -> Self {
        Self {
            address,
            provider,
            chain_id,
        }
    }
}

#[async_trait]
impl TransactionSigner for GolemBaseSigner {
    fn address(&self) -> Address {
        self.address
    }

    async fn sign(&self, _data: &[u8]) -> anyhow::Result<Signature> {
        unimplemented!()
    }
}
