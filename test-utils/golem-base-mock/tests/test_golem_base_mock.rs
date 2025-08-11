use alloy::primitives::{Address, U256};
use alloy::transports::http::reqwest::Url;
use bigdecimal::BigDecimal;
use golem_base_mock::{create_test_mock_server, get_default_mock_server_url, GolemBaseMockServer};
use golem_base_sdk::{entity::Create, GolemBaseClient};
use golem_base_test_utils::init_logger;
use std::str::FromStr;

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

    // Test basic functionality
    let chain_id = client.get_chain_id().await?;
    assert_eq!(chain_id, 31337);

    let accounts = client.account_sync().await?;
    assert!(!accounts.is_empty());

    if let Some(&account) = accounts.first() {
        let balance = client.get_balance(account).await?;
        assert!(balance > BigDecimal::from(0));
    }

    // Test basic entity creation
    let test_data = b"Hello, GolemBase!";
    let create = Create {
        btl: 100,
        data: test_data.to_vec().into(),
        string_annotations: vec![],
        numeric_annotations: vec![],
    };
    let result = client.create_entities(vec![create]).await.unwrap();
    log::info!("Created {} entities", result.len());

    // Test 2: Custom mock server configuration
    log::info!("Testing custom mock server configuration...");
    let test_account = Address::from_str("0x70997970C51812dc3A010C7d01b50e0d17dc79C8")?;

    let server = GolemBaseMockServer::new()
        .with_chain_id(1337)
        .with_accounts(vec![test_account])
        .with_balance(test_account, U256::from(1000000000000000000000u128));

    let _custom_server = server.start("127.0.0.1:8546".parse()?).await?;
    let custom_rpc_url = Url::parse("http://127.0.0.1:8546")?;
    let custom_client = GolemBaseClient::new(custom_rpc_url)?;

    let custom_chain_id = custom_client.get_chain_id().await?;
    assert_eq!(custom_chain_id, 1337);

    let custom_accounts = custom_client.account_sync().await?;
    assert_eq!(custom_accounts.len(), 1);
    assert_eq!(custom_accounts[0], test_account);

    let custom_balance = custom_client.get_balance(test_account).await?;
    assert_eq!(custom_balance, BigDecimal::from(1000));

    log::info!("✅ All GolemBase mock tests completed successfully!");
    Ok(())
}
