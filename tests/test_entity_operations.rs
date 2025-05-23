use anyhow::Result;
use bigdecimal::BigDecimal;
use serial_test::serial;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

use golem_base_sdk::{
    client::GolemBaseClient,
    entity::{Create, Update},
};

const GOLEM_BASE_URL: &str = "http://localhost:8545";

fn init_logger(should_init: bool) {
    if should_init {
        let _ = env_logger::try_init();
    }
}

#[tokio::test]
#[serial]
async fn test_create_and_retrieve_entry() -> Result<()> {
    init_logger(false);

    let client = GolemBaseClient::new(Url::parse(GOLEM_BASE_URL)?)?;
    let account = client.account_generate("test123").await?;
    let fund_tx = client.fund(account, BigDecimal::from(1)).await?;

    log::info!("Account {account} funded with transaction: {fund_tx}");

    let start_block = client.get_current_block_number().await?;
    log::info!("Starting at block: {start_block}");

    let test_payload = b"test payload".to_vec();
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let entry = Create::new(test_payload.clone(), 1000)
        .annotate_string("test_type", "Test")
        .annotate_number("test_timestamp", timestamp);

    let entry_id = client.create_entry(account, entry).await?;
    log::info!("Entry created with ID: 0x{entry_id:x}");

    let entry_str = client.cat(entry_id).await?;
    log::info!("Retrieved entry 0x{entry_id:x}: {entry_str}");
    assert_eq!(entry_str, String::from_utf8(test_payload)?);

    let metadata = client.get_entity_metadata(entry_id).await?;
    log::info!("Retrieved metadata for entry 0x{entry_id:x}: {metadata:?}");

    assert_eq!(metadata.string_annotations[0].value, "Test");
    assert_eq!(metadata.numeric_annotations[0].value, timestamp);
    assert_eq!(metadata.owner, account);
    // Entry should be created in start_block + 1.
    assert_eq!(metadata.expires_at_block.unwrap(), start_block + 1001);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_entity_operations() -> Result<()> {
    init_logger(false);

    let client = GolemBaseClient::new(Url::parse(GOLEM_BASE_URL)?)?;
    let account = client.account_generate("test123").await?;
    let fund_tx = client.fund(account, BigDecimal::from(1)).await?;

    log::info!("Account {account} funded with transaction: {fund_tx}");

    // Create first entity
    let payload1 = b"first entity".to_vec();
    let timestamp1 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let entry1 = Create::new(payload1.clone(), 1000)
        .annotate_string("test_type", "First")
        .annotate_number("test_timestamp", timestamp1);

    let entry1_id = client.create_entry(account, entry1).await?;
    log::info!("First entry created with ID: 0x{entry1_id:x}");

    // Create second entity
    let payload2 = b"second entity".to_vec();
    let timestamp2 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let entry2 = Create::new(payload2.clone(), 1000)
        .annotate_string("test_type", "Second")
        .annotate_number("test_timestamp", timestamp2);

    let entry2_id = client.create_entry(account, entry2).await?;
    log::info!("Second entry created with ID: 0x{entry2_id:x}");

    // Verify both entities exist
    let entry1_str = client.cat(entry1_id).await?;
    let entry2_str = client.cat(entry2_id).await?;
    log::info!("Retrieved first entry 0x{entry1_id:x}: {entry1_str}");
    log::info!("Retrieved second entry 0x{entry2_id:x}: {entry2_str}");
    assert_eq!(entry1_str, String::from_utf8(payload1)?);
    assert_eq!(entry2_str, String::from_utf8(payload2)?);

    // Update first entity
    let updated_payload = b"updated first entity".to_vec();
    let updated_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let update = Update::new(entry1_id, updated_payload.clone(), 1000)
        .annotate_string("test_type", "Updated")
        .annotate_number("test_timestamp", updated_timestamp);

    client.update_entry(account, update).await?;
    log::info!("First entry 0x{entry1_id:x} updated");

    // Verify first entity was updated
    let updated_str = client.cat(entry1_id).await?;
    log::info!("Retrieved updated first entry 0x{entry1_id:x}: {updated_str}");
    assert_eq!(updated_str, String::from_utf8(updated_payload.clone())?);

    // Remove second entity
    client.remove_entries(account, vec![entry2_id]).await?;
    log::info!("Second entry 0x{entry2_id:x} removed");

    // Verify second entity was removed
    let result = client.get_entity_metadata(entry2_id).await;
    assert!(
        result.is_err(),
        "Second entity 0x{entry2_id:x} should be removed. Instead got metadata: {:?}",
        result.unwrap()
    );
    assert!(result.unwrap_err().to_string().contains("not found"));

    // Verify first entity still exists
    let final_str = client.cat(entry1_id).await?;
    log::info!("Retrieved final first entry 0x{entry1_id:x}: {final_str}");
    assert_eq!(final_str, String::from_utf8(updated_payload)?);

    Ok(())
}
