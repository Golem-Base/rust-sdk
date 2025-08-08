use alloy::primitives::Address;
use alloy::signers::local::PrivateKeySigner;
use alloy::transports::http::reqwest::Url;
use bigdecimal::BigDecimal;
use golem_base_mock::{create_test_mock_server, get_default_mock_server_url};
use golem_base_sdk::{GolemBaseClient, Hash};
use std::str::FromStr;

/// Integration test that demonstrates using the GolemBase mock server with GolemBaseClient
#[tokio::test]
async fn test_golem_base_mock_with_client() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    // Start the mock server
    let _mock_server = create_test_mock_server().await?;

    // Create a GolemBaseClient that connects to the mock server
    let rpc_url = Url::parse(&get_default_mock_server_url())?;
    let client = GolemBaseClient::new(rpc_url)?;

    // Test basic functionality
    let chain_id = client.get_chain_id().await?;
    assert_eq!(chain_id, 31337);

    // Test getting accounts
    let accounts = client.account_sync().await?;
    assert!(!accounts.is_empty());

    // Test getting balance for the first account
    if let Some(&account) = accounts.first() {
        let balance = client.get_balance(account).await?;
        assert!(balance > BigDecimal::from(0));
    }

    // Test creating an entity
    let test_data = b"Hello, GolemBase!";

    // Create a test entity
    let create = golem_base_sdk::entity::Create {
        btl: 100, // Block time to live
        data: test_data.to_vec().into(),
        string_annotations: vec![],
        numeric_annotations: vec![],
    };

    // This would normally create the entity, but in the mock it will just return a transaction hash
    let result = client.create_entities(vec![create]).await;
    assert!(result.is_ok());

    println!("✅ GolemBase mock test completed successfully!");
    Ok(())
}

/// Example of how to use the mock server in integration tests
#[tokio::test]
async fn test_mock_integration() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Start mock server
    let _mock_server = create_test_mock_server().await?;

    // Create client with a specific private key for testing
    let private_key = Hash::from_slice(&[1u8; 32]); // Test private key
    let signer = PrivateKeySigner::from_bytes(&private_key)?;
    let rpc_url = Url::parse(&get_default_mock_server_url())?;

    let client = GolemBaseClient::builder()
        .wallet(signer)
        .rpc_url(rpc_url)
        .build();

    // Test entity creation
    let test_entity_data = b"Test entity data";
    let create = golem_base_sdk::entity::Create {
        btl: 50,
        data: test_entity_data.to_vec().into(),
        string_annotations: vec![golem_base_sdk::entity::StringAnnotation::new(
            "test_key",
            "test_value",
        )],
        numeric_annotations: vec![golem_base_sdk::entity::NumericAnnotation::new(
            "test_num", 42u64,
        )],
    };

    let results = client.create_entities(vec![create]).await?;
    println!("Created {} entities", results.len());

    // Test entity retrieval (this would work if the mock was properly implemented)
    for result in results {
        println!("Entity key: {:?}", result.entity_key);
        println!("Expiration block: {}", result.expiration_block);
    }

    Ok(())
}

/// Test that demonstrates custom mock server configuration
#[tokio::test]
async fn test_custom_mock_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use alloy::primitives::U256;
    use golem_base_mock::GolemBaseMockServer;

    // Create a custom mock server
    let test_account = Address::from_str("0x70997970C51812dc3A010C7d01b50e0d17dc79C8")?;

    let server = GolemBaseMockServer::new()
        .with_chain_id(1337)
        .with_accounts(vec![test_account])
        .with_syncing(false);

    // Start the server
    let mut server = server.start("127.0.0.1:8546".parse()?).await?;

    // Set custom balance
    server
        .state_mut()
        .data
        .write()
        .await
        .balances
        .insert(test_account, U256::from(1000000000000000000000u128));

    // Create a client that connects to the custom mock server
    let rpc_url = Url::parse("http://127.0.0.1:8546")?;
    let client = GolemBaseClient::new(rpc_url)?;

    // Test the custom configuration
    let chain_id = client.get_chain_id().await?;
    assert_eq!(chain_id, 1337);

    let accounts = client.account_sync().await?;
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0], test_account);

    let balance = client.get_balance(test_account).await?;
    assert_eq!(balance, BigDecimal::from(1000));

    println!("✅ Custom mock server test completed successfully!");
    Ok(())
}
