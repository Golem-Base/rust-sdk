use bigdecimal::BigDecimal;
use golem_base_mock::{
    controller::{CallOverride, CallResponse},
    GolemBaseMockServer,
};
use golem_base_sdk::GolemBaseClient;
use golem_base_test_utils::{create_test_account, init_logger};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_resilient_client_retry() -> anyhow::Result<()> {
    init_logger(true);

    let mock = GolemBaseMockServer::create_test_mock_server().await?;
    let client = GolemBaseClient::new(mock.url().clone())?;
    let account = create_test_account(&client).await.unwrap();

    let _callback = mock.controller().override_rpc(
        "eth_getBalance",
        CallOverride::Once(CallResponse::Error("error sending request".to_string())),
    );

    let balance = client.get_balance(account).await.unwrap();
    assert_eq!(balance, BigDecimal::from(1));

    log::info!("✅ All GolemBase mock tests completed successfully!");
    Ok(())
}
