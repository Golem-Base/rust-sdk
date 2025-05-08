use anyhow::Result;
use bigdecimal::BigDecimal;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

use golem_base_sdk::{client::GolemBaseClient, entity::Create};

const GOLEM_BASE_URL: &str = "http://localhost:8545";

#[tokio::test]
async fn test_create_and_retrieve_entry() -> Result<()> {
    let _ = env_logger::try_init();
    let client = GolemBaseClient::new(Url::parse(GOLEM_BASE_URL)?)?;

    let account = client.account_generate("test123").await?;
    let fund_tx = client.fund(account, BigDecimal::from(1)).await?;

    log::info!("Account {account:?} funded with transaction: {fund_tx:?}");

    let test_payload = b"test payload".to_vec();
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let entry = Create::new(test_payload.clone(), 1000)
        .annotate_string("test_type", "Test")
        .annotate_number("test_timestamp", timestamp);

    let entry_id = client.create_entry(account, entry).await?;
    log::info!("Entry created with ID: {:?}", entry_id);

    let entry_str = client.cat(entry_id.clone()).await?;
    log::info!("Retrieved entry: {}", entry_str);
    assert_eq!(entry_str, String::from_utf8(test_payload)?);

    let metadata = client.get_entity_metadata(entry_id).await?;
    log::info!("Retrieved metadata: {:?}", metadata);

    assert_eq!(metadata.string_annotations[0].value, "Test");
    assert_eq!(metadata.numeric_annotations[0].value, timestamp);
    Ok(())
}
