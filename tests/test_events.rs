use alloy::transports::http::reqwest::Url;
use bigdecimal::BigDecimal;
use futures::StreamExt;
use golem_base_sdk::client::GolemBaseClient;
use golem_base_sdk::entity::{Create, Update};
use golem_base_sdk::events::Event;
use serial_test::serial;
use std::time::Duration;

const GOLEM_BASE_URL: &str = "http://localhost:8545";
const TEST_TTL: u64 = 30;

fn init_logger(should_init: bool) {
    if should_init {
        let _ = env_logger::try_init();
    }
}

#[tokio::test]
#[serial]
async fn test_event_listening() {
    init_logger(false);

    let url = Url::parse(GOLEM_BASE_URL).unwrap();
    let client = GolemBaseClient::new(url).unwrap();
    let account = client.account_generate("test123").await.unwrap();
    client.fund(account, BigDecimal::from(1)).await.unwrap();

    // Start listening for events
    let mut event_stream = client.listen_for_events().await.unwrap();

    // Create a test entity
    let create = Create::from_string("test payload", TEST_TTL);
    let entity_id = client.create_entry(account, create).await.unwrap();

    // Wait for and verify EntityCreated event
    let event = event_stream.next().await.unwrap().unwrap();
    match event {
        Event::EntityCreated { entity_id: id, .. } => {
            assert_eq!(id, entity_id);
        }
        _ => panic!("Expected EntityCreated event"),
    }

    // Update the entity
    let update = Update::from_string(entity_id, "test payload", TEST_TTL);
    client.update_entry(account, update).await.unwrap();

    // Wait for and verify EntityUpdated event
    let event = event_stream.next().await.unwrap().unwrap();
    match event {
        Event::EntityUpdated { entity_id: id, .. } => {
            assert_eq!(id, entity_id);
        }
        _ => panic!("Expected EntityUpdated event"),
    }

    // Delete the entity
    client
        .remove_entries(account, vec![entity_id])
        .await
        .unwrap();

    // Wait for and verify EntityRemoved event
    let event = event_stream.next().await.unwrap().unwrap();
    match event {
        Event::EntityRemoved { entity_id: id, .. } => {
            assert_eq!(id, entity_id);
        }
        _ => panic!("Expected EntityRemoved event"),
    }
}

#[tokio::test]
#[serial]
async fn test_event_listening_with_timeout() {
    init_logger(false);

    let url = Url::parse(GOLEM_BASE_URL).unwrap();
    let client = GolemBaseClient::new(url).unwrap();
    let account = client.account_generate("test123").await.unwrap();
    client.fund(account, BigDecimal::from(1)).await.unwrap();

    // Start listening for events
    let mut event_stream = client.listen_for_events().await.unwrap();

    // Create a test entity
    let create = Create::from_string("test payload", TEST_TTL);
    let entity_id = client.create_entry(account, create).await.unwrap();

    // Wait for event with timeout
    let event = tokio::time::timeout(Duration::from_secs(5), event_stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    match event {
        Event::EntityCreated { entity_id: id, .. } => {
            assert_eq!(id, entity_id);
        }
        _ => panic!("Expected EntityCreated event"),
    }
}
