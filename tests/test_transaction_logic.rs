use alloy::primitives::U256;
use golem_base_mock::{
    controller::{CallOverride, CallResponse},
    GolemBaseMockServer,
};
use golem_base_sdk::{client::TransactionConfig, entity::Create, GolemBaseClient};
use golem_base_test_utils::{
    create_test_account,
    golembase::{Config, GolemBaseContainer},
    init_logger,
};
use serial_test::serial;

const NUM_ITERATIONS: usize = 50;

#[tokio::test]
#[serial]
async fn test_transaction_random_errors() -> anyhow::Result<()> {
    init_logger(false);

    let mock = GolemBaseMockServer::create_test_mock_server().await?;
    let ctrl = mock.controller();
    let client = GolemBaseClient::new(mock.url().clone())?;
    let account = create_test_account(&client).await.unwrap();

    let _callback = ctrl.global_override(CallOverride::Always(CallResponse::FailEachNth {
        error: "error sending request".to_string(),
        frequency: 2,
    }));

    for i in 0..NUM_ITERATIONS {
        let create = Create::from_string("Hello, GolemBase!", 100)
            .annotate_string("test_type", "Test")
            .annotate_number("test_timestamp", 1234567890)
            .annotate_number("iteration", i as u64);

        let result = client.create_entry(account, create).await.unwrap();
        log::info!("Created entity {result} in iteration {i}...");
    }

    log::info!(
        "✅ Successfully created {} entities with deterministic error handling (every 3rd request fails)!",
        NUM_ITERATIONS
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_transaction_indexing_in_progress() -> anyhow::Result<()> {
    init_logger(false);

    let mock = GolemBaseMockServer::create_test_mock_server().await?;
    let ctrl = mock.controller();
    let client = GolemBaseClient::new(mock.url().clone())?;
    let account = create_test_account(&client).await.unwrap();

    let _callback = ctrl.override_rpc(
        "eth_getTransactionReceipt",
        CallOverride::NTimes {
            response: CallResponse::Error("transaction indexing is in progress".to_string()),
            n: 4,
        },
    );

    let create = Create::from_string("Hello, GolemBase!", 100)
        .annotate_string("test_type", "Test")
        .annotate_number("test_timestamp", 1234567890);

    let result = client.create_entry(account, create).await.unwrap();
    log::info!("Created entity {result}...");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_transaction_nonce_too_low() -> anyhow::Result<()> {
    init_logger(false);

    let mock = GolemBaseMockServer::create_test_mock_server().await?;
    let ctrl = mock.controller();
    let client = GolemBaseClient::new(mock.url().clone())?;
    let account = create_test_account(&client).await.unwrap();

    let create = Create::from_string("Hello, GolemBase!", 100);
    let result = client.create_entry(account, create).await.unwrap();
    log::info!("Created first entity {result}...");

    let nonce = client
        .get_rpc_client()
        .get_transaction_count(account)
        .await
        .unwrap();

    // Simulating situation when we have RPC switch and the new instance doesn't know about
    // the previous transaction yet.
    ctrl.override_rpc(
        "eth_getTransactionCount",
        CallOverride::Once(CallResponse::custom(&U256::from(nonce - 1)).unwrap()),
    );

    log::info!("Creating entity with nonce too low...");
    let create = Create::from_string("Hello 2", 100);
    let result = client.create_entry(account, create).await.unwrap();
    log::info!("Created entity {result}...");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_transaction_nonce_too_low_new_client() -> anyhow::Result<()> {
    init_logger(false);

    let mock = GolemBaseMockServer::create_test_mock_server().await?;
    let ctrl = mock.controller();
    let client = GolemBaseClient::new(mock.url().clone())?;
    let account = create_test_account(&client).await.unwrap();

    let create = Create::from_string("Hello, GolemBase!", 100);
    let result = client.create_entry(account, create).await.unwrap();
    log::info!("Created first entity {result}...");

    let nonce = client
        .get_rpc_client()
        .get_transaction_count(account)
        .await
        .unwrap();

    // Simulating situation when we have RPC switch and the new instance doesn't know about
    // the previous transaction yet.
    ctrl.override_rpc(
        "eth_getTransactionCount",
        CallOverride::Once(CallResponse::custom(&U256::from(nonce - 1)).unwrap()),
    );

    // New client doesn't have previous nonce stored, so it will get the error.
    // This simulates scenario when application using the client is restarted.
    let client2 = GolemBaseClient::new(mock.url().clone())?;
    client2.account_load(account, "test123").await?;

    log::info!("Creating entity with nonce too low...");
    let create = Create::from_string("Hello 2", 100);
    let result = client2.create_entry(account, create).await.unwrap();
    log::info!("Created entity {result}...");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_transaction_wait_for_confirmations() -> anyhow::Result<()> {
    init_logger(false);

    let mock = GolemBaseMockServer::create_test_mock_server().await?;
    let client = GolemBaseClient::new(mock.url().clone())?.override_config(TransactionConfig {
        required_confirmations: 1,
        ..TransactionConfig::default()
    });
    let account = create_test_account(&client).await.unwrap();

    let create = Create::from_string("Hello, GolemBase!", 100);
    let result = client.create_entry(account, create).await.unwrap();
    log::info!("Created first entity {result}...");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_transaction_rpc_restart() -> anyhow::Result<()> {
    init_logger(false);

    let container = GolemBaseContainer::new(Config::default().with_port(33221)).await?;
    let client = GolemBaseClient::new(container.get_url()?)?;
    let account = create_test_account(&client).await.unwrap();

    let create = Create::from_string("Hello, GolemBase!", 100);
    let result = client.create_entry(account, create).await.unwrap();
    log::info!("Created first entity {result}...");

    // Restarting container to check if transaction logic will be able to handle the situation.
    container.pause().await.unwrap();

    log::info!("Creating entity when RPC is down... It should fail.");
    let result = client
        .create_entry(account, Create::from_string("Hello 2", 100))
        .await;
    assert!(result.is_err());
    container.unpause().await.unwrap();

    log::info!("Creating entity after RPC restart... It should succeed.");
    let create = Create::from_string("Hello 3", 100);
    let result = client.create_entry(account, create).await.unwrap();
    log::info!("Created entity {result}...");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_transaction_no_rpc_available() -> anyhow::Result<()> {
    init_logger(false);

    let container = GolemBaseContainer::new(Config::default()).await?;
    let client = GolemBaseClient::new(container.get_url()?)?;
    let account = create_test_account(&client).await.unwrap();

    let create = Create::from_string("Hello, GolemBase!", 100);
    let result = client.create_entry(account, create).await.unwrap();
    log::info!("Created first entity {result}...");

    // Stop the container to simulate RPC downtime
    container.stop().await.unwrap();

    log::info!("Test must ensure, that creating entity call won't hang in case of RPC downtime.");
    log::info!("Creating entity when RPC is down... It should fail.");
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        client.create_entry(account, Create::from_string("Hello 2", 100)),
    )
    .await
    .expect("Call timed out - function should have failed internally before our timeout");
    assert!(result.is_err());

    Ok(())
}

// - Replacment transaction underpriced and gas bumping
// - Pending transaction stacked in mempool
// - How to react to chain id change (network re-deploy)
// - Handle network re-deploy during client work (for example nonce is reset back to 0)
// - Handle RPC downtime or inaccesibility
