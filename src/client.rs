use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use alloy::eips::BlockNumberOrTag;
use alloy::primitives::{Address, B256};
use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use alloy::rpc::client::ClientRef;
use alloy::rpc::types::SyncStatus;
use alloy::signers::local::PrivateKeySigner;
use alloy::transports::http::reqwest::Url;
use bigdecimal::BigDecimal;
use bon::bon;
use bytes::Bytes;

use crate::account::{Account, TransactionSigner};
use crate::entity::{Create, GolemBaseTransaction, Hash, Update};
use crate::rpc::Error;
use crate::signers::{GolemBaseSigner, InMemorySigner};
use crate::utils::wei_to_eth;
use log;

/// A client for interacting with the GolemBase system.
#[derive(Clone)]
pub struct GolemBaseClient {
    /// The underlying provider for making RPC calls
    pub(crate) provider: DynProvider,
    pub(crate) accounts: Arc<RwLock<HashMap<Address, Account>>>,
    /// The URL of the GolemBase endpoint
    pub(crate) rpc_url: Url,
    /// The Ethereum address of the client owner.
    pub(crate) wallet: PrivateKeySigner,
}

#[bon]
impl GolemBaseClient {
    #[builder]
    pub fn builder(wallet: PrivateKeySigner, rpc_url: Url) -> Self {
        let provider = ProviderBuilder::new()
            .connect_http(rpc_url.clone())
            .erased();

        Self {
            provider,
            accounts: Arc::new(RwLock::new(HashMap::new())),
            rpc_url,
            wallet,
        }
    }

    /// Gets the underlying Reqwest client used for HTTP requests.
    pub fn get_reqwest_client(&self) -> ClientRef<'_> {
        self.provider.client()
    }

    /// Gets the Ethereum address of the client owner.
    pub fn get_owner_address(&self) -> Address {
        self.wallet.address()
    }

    /// Creates a new client
    pub fn new(endpoint: Url) -> anyhow::Result<Self> {
        Self::new_uninitialized(endpoint)
    }

    /// Creates a new client without initializing it
    pub fn new_uninitialized(endpoint: Url) -> anyhow::Result<Self> {
        let provider = ProviderBuilder::new()
            .connect_http(endpoint.clone())
            .erased();

        Ok(Self {
            provider,
            accounts: Arc::new(RwLock::new(HashMap::new())),
            rpc_url: endpoint,
            wallet: PrivateKeySigner::random(),
        })
    }

    /// Gets the chain ID from the provider
    pub async fn get_chain_id(&self) -> anyhow::Result<u64> {
        self.provider
            .get_chain_id()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get chain ID: {}", e))
    }

    /// Checks chain ID and syncs accounts with GolemBase node
    pub async fn sync_node(&self, timeout: Duration) -> anyhow::Result<()> {
        let start_time = Instant::now();
        let stop_time = start_time + timeout;

        while !self.is_synced().await? {
            if Instant::now() > stop_time {
                return Err(anyhow::anyhow!(
                    "Timeout {} while syncing node",
                    humantime::format_duration(timeout)
                ));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let chain_id = self.get_chain_id().await?;
        self.sync_golem_base_accounts(chain_id).await?;
        Ok(())
    }

    /// Registers a user-managed account with custom signer.
    pub async fn account_register(
        &self,
        signer: impl TransactionSigner + 'static,
    ) -> anyhow::Result<Address> {
        let address = signer.address();
        let chain_id = self.get_chain_id().await?;
        let mut accounts = self.accounts.write().unwrap();
        accounts.insert(
            address,
            Account::new(Box::new(signer), self.provider.clone(), chain_id),
        );
        Ok(address)
    }

    /// Generates a new local key, saves it to a keystore file, and registers it
    pub async fn account_generate(&self, password: &str) -> anyhow::Result<Address> {
        let signer = InMemorySigner::generate();
        let _path = signer
            .save(password)
            .map_err(|e| anyhow::anyhow!("Failed to save account: {e}"))?;
        self.account_register(signer).await
    }

    /// Loads a key from a raw private key file and registers it
    pub async fn account_load_file(
        &self,
        path: PathBuf,
        password: &str,
    ) -> anyhow::Result<Address> {
        // First try to load as keystore.
        let signer = match InMemorySigner::load_keystore(path.clone(), password) {
            Ok(signer) => signer,
            Err(_) => {
                // If keystore loading fails, try as raw key file
                InMemorySigner::load_raw_key(path)?
            }
        };
        self.account_register(signer).await
    }

    /// Loads a key from the default directory and registers it
    pub async fn account_load(&self, address: Address, password: &str) -> anyhow::Result<Address> {
        // This will load all available accounts from GolemBase.
        // We check only the registered accounts, because sync returns local as well.
        let all_accounts = self
            .account_sync()
            .await
            .map_err(|e| anyhow::anyhow!("Sync-ing accounts: {e}"))?;
        if self.accounts_list().contains(&address) {
            return Ok(address);
        }

        if !all_accounts.contains(&address) {
            return Err(anyhow::anyhow!(
                "Account {address} not found in available accounts"
            ));
        }

        // Try to load from local keystore if it wasn't loaded from GolemBase.
        let signer = InMemorySigner::load_by_address(address, password)?;
        self.account_register(signer).await
    }

    /// Lists all registered accounts
    pub fn accounts_list(&self) -> Vec<Address> {
        let accounts = self.accounts.read().unwrap();
        accounts.keys().cloned().collect()
    }

    /// Synchronizes accounts with GolemBase, adding any new accounts to our local state
    pub async fn account_sync(&self) -> anyhow::Result<Vec<Address>> {
        let chain_id = self.get_chain_id().await?;

        // Sync GolemBase accounts
        self.sync_golem_base_accounts(chain_id).await?;

        // Get all available accounts
        let mut all_accounts = self.accounts_list();
        let local_accounts = InMemorySigner::list_local_accounts()?;

        // Add local accounts that aren't already in the list
        for address in local_accounts {
            if !all_accounts.contains(&address) {
                all_accounts.push(address);
            }
        }

        Ok(all_accounts)
    }

    /// Gets an account's ETH balance
    pub async fn get_balance(&self, account: Address) -> anyhow::Result<BigDecimal> {
        let balance = self.provider.get_balance(account).await?;
        Ok(wei_to_eth(balance))
    }

    /// Transfers ETH from one account to another
    pub async fn transfer(
        &self,
        from: Address,
        to: Address,
        value: BigDecimal,
    ) -> anyhow::Result<B256> {
        let account = self.account_get(from)?;
        let receipt = account.transfer(to, value).await?;
        Ok(receipt.transaction_hash)
    }

    /// Funds an account with ETH
    pub async fn fund(&self, account: Address, value: BigDecimal) -> anyhow::Result<B256> {
        let account = self.account_get(account)?;
        let receipt = account.fund_account(value).await?;
        Ok(receipt.transaction_hash)
    }

    async fn sync_golem_base_accounts(&self, chain_id: u64) -> anyhow::Result<()> {
        let golem_accounts = self.list_golem_accounts().await?;
        let mut accounts = self.accounts.write().unwrap();

        for address in golem_accounts {
            self.try_insert_account(&mut accounts, address, chain_id, |address| {
                Box::new(GolemBaseSigner::new(
                    address,
                    self.provider.clone(),
                    chain_id,
                ))
            });
        }

        Ok(())
    }

    fn try_insert_account<F>(
        &self,
        accounts: &mut HashMap<Address, Account>,
        address: Address,
        chain_id: u64,
        create_signer: F,
    ) where
        F: FnOnce(Address) -> Box<dyn TransactionSigner>,
    {
        if accounts.contains_key(&address) {
            return;
        }

        let signer = create_signer(address);
        accounts.insert(
            address,
            Account::new(signer, self.provider.clone(), chain_id),
        );
    }

    /// Gets an account by its address
    pub fn account_get(&self, address: Address) -> anyhow::Result<Account> {
        let accounts = self.accounts.read().unwrap();
        accounts
            .get(&address)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Account {address} not found"))
    }

    /// Internal function to list accounts from GolemBase
    async fn list_golem_accounts(&self) -> anyhow::Result<Vec<Address>> {
        Ok(self.provider.get_accounts().await?)
    }

    /// Creates an entry using the specified account
    pub async fn create_entry(&self, account: Address, entry: Create) -> anyhow::Result<Hash> {
        let account = self.account_get(account)?;
        let tx = GolemBaseTransaction {
            creates: vec![entry],
            updates: vec![],
            deletes: vec![],
            extensions: vec![],
        };

        log::debug!("Sending storage transaction from {}", account.address());

        let receipt = account.send_db_transaction(tx).await?;
        if !receipt.status() {
            return Err(anyhow::anyhow!(
                "Transaction {} failed despite being mined.",
                receipt.transaction_hash
            ));
        }

        // Parse logs to get entity ID
        let entity_id = receipt
            .logs()
            .iter()
            .find_map(|log| {
                log::debug!("Log: {:?}", log);
                if log.topics().len() >= 2
                    && log.topics()[0] == crate::account::golem_base_storage_entity_created()
                {
                    // Second topic is the entity ID
                    Some(log.topics()[1])
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow::anyhow!("No entity ID found in transaction logs"))?;

        log::debug!("Created entity with ID: 0x{:x}", entity_id);
        Ok(entity_id)
    }

    /// Removes entries from GolemBase
    ///
    /// # Arguments:
    /// * `account` - The account address that owns the entries
    /// * `entry_ids` - The IDs of the entries to remove
    pub async fn remove_entries(
        &self,
        account: Address,
        entry_ids: Vec<Hash>,
    ) -> anyhow::Result<()> {
        if entry_ids.is_empty() {
            return Ok(());
        }

        let account = self.account_get(account)?;
        let entry_count = entry_ids.len();
        let tx = GolemBaseTransaction {
            creates: vec![],
            updates: vec![],
            deletes: entry_ids,
            extensions: vec![],
        };

        log::debug!(
            "Sending delete transaction from {} for {} entries",
            account.address(),
            entry_count
        );

        let receipt = account.send_db_transaction(tx).await?;
        if !receipt.status() {
            return Err(anyhow::anyhow!(
                "Transaction {} failed despite being mined.",
                receipt.transaction_hash
            ));
        }

        log::debug!("Successfully removed {} entries", entry_count);
        Ok(())
    }

    /// Retrieves an entry's payload from Golem Base by its ID
    pub async fn cat(&self, id: Hash) -> anyhow::Result<String> {
        let bytes = self.get_storage_value::<Bytes>(id).await?;
        Ok(String::from_utf8(bytes.to_vec()).map_err(|e| Error::UnexpectedError(e.to_string()))?)
    }

    /// Checks if the node is synced by comparing the latest block timestamp with current time
    /// Returns true if the node is synced (latest block is less than 5 minutes old)
    pub async fn is_synced(&self) -> anyhow::Result<bool> {
        let syncing = self.provider.syncing().await?;
        match syncing {
            SyncStatus::Info(sync) => {
                let current_block = sync.current_block;
                let highest_block = sync.highest_block;

                if current_block == highest_block {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            SyncStatus::None => Ok(true),
        }
    }

    /// Updates an entry using the specified account
    ///
    /// # Arguments:
    /// * `account` - The account address that owns the entry
    /// * `update` - The update operation containing new data and annotations
    pub async fn update_entry(&self, account: Address, update: Update) -> anyhow::Result<()> {
        let entity_key = update.entity_key;
        let account = self.account_get(account)?;
        let tx = GolemBaseTransaction {
            creates: vec![],
            updates: vec![update],
            deletes: vec![],
            extensions: vec![],
        };

        log::debug!(
            "Sending update transaction from {} for entry 0x{:x}",
            account.address(),
            entity_key
        );

        let receipt = account.send_db_transaction(tx).await?;
        if !receipt.status() {
            return Err(anyhow::anyhow!(
                "Transaction {} failed despite being mined.",
                receipt.transaction_hash
            ));
        }

        log::debug!("Successfully updated entry with ID: 0x{:x}", entity_key);
        Ok(())
    }

    /// Gets the current block number
    pub async fn get_current_block_number(&self) -> anyhow::Result<u64> {
        let latest_block = self
            .provider
            .get_block_by_number(BlockNumberOrTag::Latest)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to get latest block"))?;
        Ok(latest_block.header.number)
    }
}
