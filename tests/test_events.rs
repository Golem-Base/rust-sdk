use anyhow::Result;
use futures::StreamExt;
use golem_base_sdk::GolemBaseClient;
use serial_test::serial;
use std::time::Duration;
use url::Url;

use golem_base_sdk::entity::{Create, Update};
use golem_base_sdk::events::Event;
use golem_base_test_utils::{
    GOLEM_BASE_URL, TEST_TTL, cleanup_entities, create_test_account, init_logger,
};

#[tokio::test]
#[serial]
#[ignore]
async fn test_event_listening() -> Result<()> {
    init_logger(false);

    let client = GolemBaseClient::new(Url::parse(GOLEM_BASE_URL)?)?;
    let account = create_test_account(&client).await?;
    cleanup_entities(&client, account).await?;

    // Start listening for events, before we create the entity to avoid missing the event.
    let events = client.events_client().await.unwrap();
    let mut event_stream = events.events_stream().await.unwrap();

    // Create a test entity
    let create = Create::from_string("test payload", TEST_TTL);
    let entity_id = client.create_entry(account, create).await.unwrap();

    // Wait for and verify EntityCreated event
    let event = event_stream.next().await.unwrap().unwrap();
    log::info!("Event: {:?}", event);
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
    Ok(())
}

#[tokio::test]
#[serial]
#[ignore]
async fn test_event_listening_with_timeout() -> Result<()> {
    init_logger(false);

    let client = GolemBaseClient::new(Url::parse(GOLEM_BASE_URL)?)?;
    let account = create_test_account(&client).await.unwrap();
    cleanup_entities(&client, account).await.unwrap();

    // Start listening for events
    let events = client.events_client().await.unwrap();
    let mut event_stream = events.events_stream().await.unwrap();

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
    Ok(())
}
