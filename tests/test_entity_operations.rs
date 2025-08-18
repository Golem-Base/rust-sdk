use alloy::primitives::FixedBytes;
use anyhow::Result;
use bytes::Bytes;
use serial_test::serial;
use std::time::{SystemTime, UNIX_EPOCH};

use golem_base_sdk::entity::{Create, Extend, GolemBaseTransaction, Update};
use golem_base_test_utils::get_client;

#[tokio::test]
#[serial]
async fn test_create_and_retrieve_entry() -> Result<()> {
    let client = get_client()?;

    let start_block = client.get_current_block_number().await?;
    log::info!("Starting at block: {start_block}");

    let test_payload = b"test payload".to_vec();
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let create_tx = Create::new(test_payload.clone(), 1000)
        .annotate_string("test_type", "Test")
        .annotate_number("test_timestamp", timestamp);

    let tx_results = client.create_entities(vec![create_tx]).await?;
    let entity_result = &tx_results[0];
    log::info!("Entry created with ID: {entity_result:?}");

    let entry_str = client
        .get_storage_value::<Bytes>(entity_result.entity_key)
        .await?;
    log::info!("Retrieved value: {entry_str:?}");
    assert_eq!(entry_str, String::from_utf8(test_payload)?);

    let metadata = client.get_entity_metadata(entity_result.entity_key).await?;
    log::info!("Retrieved metadata: {metadata:?}");

    assert_eq!(metadata.string_annotations[0].value, "Test");
    assert_eq!(metadata.numeric_annotations[0].value, timestamp);
    assert_eq!(metadata.owner, client.get_owner_address());
    // Entry should be created in start_block + 1.
    assert_eq!(metadata.expires_at_block.unwrap(), start_block + 1001);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_entity_operations() -> Result<()> {
    let client = get_client()?;

    // Create first entity
    let payload1 = b"first entity".to_vec();
    let timestamp1 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let create1 = Create::new(payload1.clone(), 1000)
        .annotate_string("test_type", "First")
        .annotate_number("test_timestamp", timestamp1);

    let tx1_results = client.create_entities(vec![create1]).await?;
    let entity1_result = &tx1_results[0];
    log::info!("Entry created with ID: {entity1_result:?}");

    // Create second entity
    let payload2 = b"second entity".to_vec();
    let timestamp2 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let create2 = Create::new(payload2.clone(), 1000)
        .annotate_string("test_type", "Second")
        .annotate_number("test_timestamp", timestamp2);

    let tx2_results = client.create_entities(vec![create2]).await?;
    let entity2_result = &tx2_results[0];
    log::info!("Entry created with ID: {entity1_result:?}");

    // Verify both entities exist
    let entity1_str = String::from_utf8(
        client
            .get_storage_value::<Bytes>(entity1_result.entity_key)
            .await?
            .to_vec(),
    )
    .unwrap();
    let entity2_str = String::from_utf8(
        client
            .get_storage_value::<Bytes>(entity2_result.entity_key)
            .await?
            .to_vec(),
    )
    .unwrap();
    log::info!("Retrieved first entry: {entity1_str}");
    log::info!("Retrieved second entry: {entity2_str}");
    assert_eq!(entity1_str, String::from_utf8(payload1)?);
    assert_eq!(entity2_str, String::from_utf8(payload2)?);

    // Update first entity
    let updated_payload = b"updated first entity".to_vec();
    let updated_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let update = Update::new(entity1_result.entity_key, updated_payload.clone(), 1000)
        .annotate_string("test_type", "Updated")
        .annotate_number("test_timestamp", updated_timestamp);

    client.update_entities(vec![update]).await?;
    log::info!("First entry updated");

    // Verify first entity was updated
    let updated_str = String::from_utf8(
        client
            .get_storage_value::<Vec<u8>>(entity1_result.entity_key)
            .await?,
    )
    .unwrap();
    log::info!("Retrieved updated first entry: {updated_str}");
    assert_eq!(updated_str, String::from_utf8(updated_payload.clone())?);

    // Remove second entity
    client
        .delete_entities(vec![entity2_result.entity_key])
        .await?;
    log::info!("Second entry removed");

    // Verify second entity was removed
    let result = client.get_entity_metadata(entity2_result.entity_key).await;
    assert!(
        result.is_err(),
        "Second entity should be removed. Instead got metadata: {:?}",
        result.unwrap()
    );
    assert!(result.unwrap_err().to_string().contains("not found"));

    // Verify first entity still exists
    let final_str = String::from_utf8(
        client
            .get_storage_value::<Bytes>(entity1_result.entity_key)
            .await?
            .to_vec(),
    )
    .unwrap();
    log::info!("Retrieved final first entry: {final_str}");
    assert_eq!(final_str, String::from_utf8(updated_payload)?);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_concurrent_entity_creation_batch() -> Result<()> {
    let client = get_client()?;

    // Number of entities to create per task
    const ENTITIES_PER_TASK: usize = 15;

    // Spawn two tasks that will create entities concurrently
    let task1 = tokio::spawn({
        let client = client.clone();
        async move {
            let mut creates = Vec::with_capacity(ENTITIES_PER_TASK);
            for i in 0..ENTITIES_PER_TASK {
                let payload = format!("task1_entity_{i}").into_bytes();
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
                let payload = format!("task2_entity_{i}").into_bytes();
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
        let entry_str = String::from_utf8(
            client
                .get_storage_value::<Vec<u8>>(result.entity_key)
                .await?,
        )
        .unwrap();
        assert_eq!(entry_str, format!("task1_entity_{i}"));

        let metadata = client.get_entity_metadata(result.entity_key).await?;
        assert_eq!(metadata.string_annotations[0].value, "task1");
        assert_eq!(metadata.numeric_annotations[0].value, i as u64);
    }

    for (i, result) in task2_entities.iter().enumerate() {
        let entry_str = String::from_utf8(
            client
                .get_storage_value::<Vec<u8>>(result.entity_key)
                .await?,
        )
        .unwrap();
        assert_eq!(entry_str, format!("task2_entity_{i}"));

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

#[tokio::test]
#[serial]
async fn test_failed_tx_explicit_gas() -> Result<()> {
    let client = get_client()?;

    let start_block = client.get_current_block_number().await?;
    log::info!("Starting at block: {start_block}");

    let create_tx = GolemBaseTransaction::builder()
        .extensions(vec![Extend::new(FixedBytes::with_last_byte(1), 1000)])
        .gas_limit(235200)
        .build();

    let tx_results = client.send_transaction(create_tx).await;

    assert!(
        tx_results
            .unwrap_err()
            .to_string()
            .contains("Error during tx execution: ")
    );

    Ok(())
}
