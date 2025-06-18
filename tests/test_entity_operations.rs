use alloy::primitives::B256;
use anyhow::{Result, anyhow};
use dirs::config_dir;
use serial_test::serial;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

use golem_base_sdk::{
    PrivateKeySigner,
    client::GolemBaseClient,
    entity::{Create, Update},
};
use golem_base_test_utils::{GOLEM_BASE_URL, create_test_account, init_logger};

#[tokio::test]
#[serial]
async fn test_create_and_retrieve_entry() -> Result<()> {
    init_logger(false);

    let client = GolemBaseClient::new(Url::parse(GOLEM_BASE_URL)?)?;
    let account = create_test_account(&client).await?;

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
    let account = create_test_account(&client).await?;

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

fn get_client() -> Result<GolemBaseClient> {
    let mut private_key_path =
        config_dir().ok_or_else(|| anyhow!("Failed to get config directory"))?;
    private_key_path.push("golembase/private.key");
    let private_key_bytes = fs::read(&private_key_path)?;
    let private_key = B256::from_slice(&private_key_bytes);

    let signer = PrivateKeySigner::from_bytes(&private_key)
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?;
    let url = Url::parse(GOLEM_BASE_URL)?;
    let client = GolemBaseClient::builder()
        .wallet(signer)
        .rpc_url(url)
        .build();
    Ok(client)
}

#[tokio::test]
#[serial]
async fn test_concurrent_entity_creation_batch() -> Result<()> {
    init_logger(true);

    let client = get_client()?;

    // Number of entities to create per task
    const ENTITIES_PER_TASK: usize = 15;

    // Spawn two tasks that will create entities concurrently
    let task1 = tokio::spawn({
        let client = client.clone();
        async move {
            let mut creates = Vec::with_capacity(ENTITIES_PER_TASK);
            for i in 0..ENTITIES_PER_TASK {
                let payload = format!("task1_entity_{}", i).into_bytes();
                let entry = Create::new(payload, 300)
                    .annotate_string("task", "task1")
                    .annotate_number("index", i as u64);
                creates.push(entry);
            }
            let results = client.create_entities(creates).await?;
            Ok::<_, anyhow::Error>(results)
        }
    });

    let task2 = tokio::spawn({
        let client = client.clone();
        async move {
            let mut creates = Vec::with_capacity(ENTITIES_PER_TASK);
            for i in 0..ENTITIES_PER_TASK {
                let payload = format!("task2_entity_{}", i).into_bytes();
                let entry = Create::new(payload, 300)
                    .annotate_string("task", "task2")
                    .annotate_number("index", i as u64);
                creates.push(entry);
            }
            let results = client.create_entities(creates).await?;
            Ok::<_, anyhow::Error>(results)
        }
    });

    // Wait for both tasks to complete
    let (task1_results, task2_results) = tokio::join!(task1, task2);
    let task1_entities = task1_results??;
    let task2_entities = task2_results??;

    // Verify all entities were created successfully
    for (i, result) in task1_entities.iter().enumerate() {
        let entry_str = client.cat(result.entity_key).await?;
        assert_eq!(entry_str, format!("task1_entity_{}", i));

        let metadata = client.get_entity_metadata(result.entity_key).await?;
        assert_eq!(metadata.string_annotations[0].value, "task1");
        assert_eq!(metadata.numeric_annotations[0].value, i as u64);
    }

    for (i, result) in task2_entities.iter().enumerate() {
        let entry_str = client.cat(result.entity_key).await?;
        assert_eq!(entry_str, format!("task2_entity_{}", i));

        let metadata = client.get_entity_metadata(result.entity_key).await?;
        assert_eq!(metadata.string_annotations[0].value, "task2");
        assert_eq!(metadata.numeric_annotations[0].value, i as u64);
    }

    log::info!(
        "Successfully verified {} concurrent batch entity creations",
        ENTITIES_PER_TASK * 2
    );
    Ok(())
}
