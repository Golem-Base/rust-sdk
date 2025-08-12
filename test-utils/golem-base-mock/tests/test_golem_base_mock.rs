use alloy::transports::http::reqwest::Url;
use bigdecimal::BigDecimal;
use golem_base_mock::{create_test_mock_server, get_default_mock_server_url};
use golem_base_sdk::{entity::Create, GolemBaseClient};
use golem_base_test_utils::{create_test_account, init_logger};

/// Comprehensive integration test that demonstrates using the GolemBase mock server with GolemBaseClient
#[tokio::test]
async fn test_golem_base_mock_integration() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    init_logger(true);

    // Test 1: Basic functionality with default mock server
    log::info!("Testing basic functionality with default mock server...");
    let _mock_server = create_test_mock_server().await?;
    let rpc_url = Url::parse(&get_default_mock_server_url())?;
    let client = GolemBaseClient::new(rpc_url)?;

    // Are we able to get the chain id?
    let chain_id = client.get_chain_id().await?;
    assert_eq!(chain_id, 31337);

    // Create a test account with funding
    let account = create_test_account(&client).await.unwrap();
    let balance = client.get_balance(account).await?;
    assert!(balance == BigDecimal::from(1));

    // Test basic entity creation
    let test_data = b"Hello, GolemBase!";
    let create = Create::new(test_data.to_vec(), 100)
        .annotate_string("test_type", "Test")
        .annotate_number("test_timestamp", 1234567890);

    let result = client.create_entry(account, create).await.unwrap();
    log::info!("Created {} entities", result);

    log::info!("✅ All GolemBase mock tests completed successfully!");
    Ok(())
}
