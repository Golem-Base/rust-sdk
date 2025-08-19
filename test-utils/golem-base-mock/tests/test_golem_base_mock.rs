use bigdecimal::BigDecimal;
use golem_base_mock::GolemBaseMockServer;
use golem_base_sdk::{entity::Create, GolemBaseClient};
use golem_base_test_utils::{create_test_account, init_logger};
use serial_test::serial;

/// Comprehensive integration test that demonstrates using the GolemBase mock server with GolemBaseClient
#[tokio::test]
#[serial]
async fn test_golem_base_mock_integration() -> anyhow::Result<()> {
    init_logger(false);

    // Test 1: Basic functionality with default mock server
    log::info!("Testing basic functionality with default mock server...");
    let server = GolemBaseMockServer::create_test_mock_server().await?;
    let client = GolemBaseClient::new(server.url().clone())?;

    // Are we able to get the chain id?
    let chain_id = client.get_chain_id().await?;
    assert_eq!(chain_id, 1337);

    // Create a test account with funding
    log::info!("Creating test account and funding it...");
    let account = create_test_account(&client).await.unwrap();
    let balance = client.get_balance(account).await?;
    assert!(balance == BigDecimal::from(1));

    log::info!("Account {account} created with balance {balance}");

    // Test basic entity creation
    log::info!("Creating test entity...");

    let test_data = b"Hello, GolemBase!";
    let create = Create::new(test_data.to_vec(), 100)
        .annotate_string("test_type", "Test")
        .annotate_number("test_timestamp", 1234567890);

    let result = client.create_entry(account, create).await.unwrap();

    log::info!("Created entity {result}...");

    log::info!("Retrieving storage value for entity key {result}...");
    let storage_value = client.get_storage_value::<Vec<u8>>(result).await.unwrap();
    let storage_string = String::from_utf8(storage_value.clone()).unwrap();
    log::info!("Storage value: {}", storage_string);
    assert_eq!(storage_value, test_data);

    log::info!("Querying entities by string annotation 'test_type = \"Test\"'...");
    let test_type_results = client.query_entities("test_type = \"Test\"").await.unwrap();
    log::info!(
        "Found {} entities with test_type = 'Test'",
        test_type_results.len()
    );
    assert_eq!(test_type_results.len(), 1);
    assert_eq!(test_type_results[0].key, result);
    assert_eq!(test_type_results[0].value, test_data.as_slice());

    log::info!("Querying entities by numeric annotation 'test_timestamp = 1234567890'...");
    let timestamp_results = client
        .query_entities("test_timestamp = 1234567890")
        .await
        .unwrap();
    log::info!(
        "Found {} entities with test_timestamp = 1234567890",
        timestamp_results.len()
    );
    assert_eq!(timestamp_results.len(), 1);
    assert_eq!(timestamp_results[0].key, result);

    log::info!("Querying entity keys by annotation...");
    let test_type_keys = client
        .query_entity_keys("test_type = \"Test\"")
        .await
        .unwrap();
    assert_eq!(test_type_keys.len(), 1);
    assert_eq!(test_type_keys[0], result);

    log::info!("Getting entity metadata...");
    let metadata = client.get_entity_metadata(result).await.unwrap();
    log::info!("Entity metadata: {:?}", metadata);
    assert_eq!(metadata.owner, account);
    assert_eq!(metadata.string_annotations.len(), 1);
    assert_eq!(metadata.numeric_annotations.len(), 1);

    log::info!("✅ All GolemBase mock tests completed successfully!");
    Ok(())
}
