use golem_base_mock::{
    controller::{CallOverride, CallResponse},
    GolemBaseMockServer,
};
use golem_base_sdk::{entity::Create, GolemBaseClient};
use golem_base_test_utils::{create_test_account, init_logger};
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
