use golem_base_mock::GolemBaseMockServer;
use golem_base_sdk::{entity::Create, GolemBaseClient};
use golem_base_test_utils::{create_test_account, init_logger};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_transaction_retry() -> anyhow::Result<()> {
    init_logger(false);

    let mock = GolemBaseMockServer::create_test_mock_server().await?;
    let client = GolemBaseClient::new(mock.url().clone())?;

    // Create a test account with funding
    log::info!("Creating test account and funding it...");
    let account = create_test_account(&client).await.unwrap();

    // Test basic entity creation
    log::info!("Creating test entity...");

    let test_data = b"Hello, GolemBase!";
    let create = Create::new(test_data.to_vec(), 100)
        .annotate_string("test_type", "Test")
        .annotate_number("test_timestamp", 1234567890);

    let result = client.create_entry(account, create).await.unwrap();

    log::info!("Created entity {result}...");

    log::info!("✅ All GolemBase mock tests completed successfully!");
    Ok(())
}
