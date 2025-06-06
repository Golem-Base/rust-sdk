use alloy::primitives::Address;
use anyhow::Result;
use bigdecimal::BigDecimal;

use golem_base_sdk::client::GolemBaseClient;

/// Default URL for GolemBase node in tests
pub const GOLEM_BASE_URL: &str = "https://kaolin.holesky.golem-base.io/rpc";

/// Default TTL value for test entities
pub const TEST_TTL: u64 = 30;

/// Initializes the logger for tests
pub fn init_logger(should_init: bool) {
    if should_init {
        let _ = env_logger::try_init();
    }
}

/// Removes all existing entities from the GolemBase node
pub async fn cleanup_entities(client: &GolemBaseClient, account: Address) -> Result<()> {
    let all_entity_keys = client.get_all_entity_keys().await?;
    log::info!("Removing all existing entities: {:?}", all_entity_keys);
    if !all_entity_keys.is_empty() {
        client.remove_entries(account, all_entity_keys).await?;
    }
    Ok(())
}

/// Creates a new test account with initial funding
pub async fn create_test_account(client: &GolemBaseClient) -> Result<Address> {
    let account = client.account_generate("test123").await?;
    let fund_tx = client.fund(account, BigDecimal::from(1)).await?;

    log::info!("Account {account} funded with transaction: {fund_tx}");
    Ok(account)
}
