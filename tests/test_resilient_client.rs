use bigdecimal::BigDecimal;
use golem_base_mock::{
    controller::{CallOverride, CallResponse},
    GolemBaseMockServer,
};
use golem_base_sdk::GolemBaseClient;
use golem_base_test_utils::{create_test_account, init_logger};
use serial_test::serial;

/// Test validates proper handling of `error sending request` error.
#[tokio::test]
#[serial]
async fn test_resilient_client_retry() -> anyhow::Result<()> {
    init_logger(true);

    let mock = GolemBaseMockServer::create_test_mock_server().await?;
    let client = GolemBaseClient::new(mock.url().clone())?;
    let account = create_test_account(&client).await.unwrap();

    log::info!("Scenario 1: We should retry RPC call after getting `error sending request` error.");
    log::info!("Scenario checks if we are able to handle single error.");
    let _callback = mock.controller().override_rpc(
        "eth_getBalance",
        CallOverride::Once(CallResponse::Error("error sending request".to_string())),
    );

    let balance = client.get_balance(account).await.unwrap();
    assert_eq!(balance, BigDecimal::from(1));

    log::info!("Scenario 2: We should retry RPC call at least 2 times.");
    let _callback = mock.controller().override_rpc(
        "eth_getBalance",
        CallOverride::NTimes {
            n: 2,
            response: CallResponse::Error("error sending request".to_string()),
        },
    );

    let balance = client.get_balance(account).await.unwrap();
    assert_eq!(balance, BigDecimal::from(1));

    log::info!("Scenario 3: We should make maximum 3 RPC call attempts.");
    log::info!("Scenario checks if we will get error response after 3 attempts.");
    let _callback = mock.controller().override_rpc(
        "eth_getBalance",
        CallOverride::NTimes {
            n: 4,
            response: CallResponse::Error("error sending request".to_string()),
        },
    );

    let result = client.get_balance(account).await;
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("error sending request"));

    log::info!("✅ All GolemBase mock tests completed successfully!");
    Ok(())
}
