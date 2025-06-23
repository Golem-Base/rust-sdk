use anyhow::Result;
use futures::StreamExt;
use serial_test::serial;
use std::future;
use std::time::Duration;
use url::Url;

use golem_base_sdk::entity::{Create, Extend, Update};
use golem_base_sdk::events::{Event, EventsClient};
use golem_base_test_utils::{GOLEM_BASE_WS_URL, get_client};

#[tokio::test]
#[serial]
async fn test_event_listening() -> Result<()> {
    let client = get_client()?;

    // Start listening for events, before we create the entity to avoid missing the event.
    let events = EventsClient::new(Url::parse(GOLEM_BASE_WS_URL).unwrap())
        .await
        .unwrap();

    let mut event_stream = events.events_stream().await.unwrap();

    // Create a test entity
    let create = Create::from_string("test payload", 30);
    let entities = client.create_entities(vec![create]).await.unwrap();
    let entity = entities[0].clone();

    // We fast-forward the stream to the event that we are expecting
    event_stream = Box::pin(event_stream.skip_while(move |event| {
        if let Ok(Event::EntityCreated { entity_id, .. }) = event {
            // When the entity matches, we stop skipping
            future::ready(entity_id != &entity.entity_key)
        } else {
            future::ready(true)
        }
    }));

    // Wait for and verify EntityCreated event
    let event = tokio::time::timeout(Duration::from_secs(5), event_stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match event {
        Event::EntityCreated { entity_id: id, .. } => {
            assert_eq!(id, entity.entity_key);
        }
        event => panic!(
            "Expected EntityCreated for key {} event, but got: {event:?}",
            entity.entity_key
        ),
    }

    // Update the entity
    let update = Update::from_string(entity.entity_key, "test payload", 30);
    client.update_entities(vec![update]).await.unwrap();

    event_stream = Box::pin(event_stream.skip_while(move |event| {
        if let Ok(Event::EntityUpdated { entity_id, .. }) = event {
            future::ready(entity_id != &entity.entity_key)
        } else {
            future::ready(true)
        }
    }));

    // Wait for and verify EntityUpdated event
    let event = tokio::time::timeout(Duration::from_secs(5), event_stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match event {
        Event::EntityUpdated { entity_id: id, .. } => {
            assert_eq!(id, entity.entity_key);
        }
        event => panic!(
            "Expected EntityUpdated for key {} event, but got: {event:?}",
            entity.entity_key
        ),
    }

    // Extend the entity
    let extend = Extend::new(entity.entity_key, 30);
    client.extend_entities(vec![extend]).await.unwrap();

    event_stream = Box::pin(event_stream.skip_while(move |event| {
        if let Ok(Event::EntityExtended { entity_id, .. }) = event {
            future::ready(entity_id != &entity.entity_key)
        } else {
            future::ready(true)
        }
    }));

    // Wait for and verify EntityUpdated event
    let event = tokio::time::timeout(Duration::from_secs(5), event_stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match event {
        Event::EntityExtended { entity_id: id, .. } => {
            assert_eq!(id, entity.entity_key);
        }
        event => panic!(
            "Expected EntityUpdated for key {} event, but got: {event:?}",
            entity.entity_key
        ),
    }

    // Delete the entity
    client
        .delete_entities(vec![entity.entity_key])
        .await
        .unwrap();

    event_stream = Box::pin(event_stream.skip_while(move |event| {
        if let Ok(Event::EntityRemoved { entity_id, .. }) = event {
            future::ready(entity_id != &entity.entity_key)
        } else {
            future::ready(true)
        }
    }));

    // Wait for and verify EntityRemoved event
    let event = tokio::time::timeout(Duration::from_secs(5), event_stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match event {
        Event::EntityRemoved { entity_id: id, .. } => {
            assert_eq!(id, entity.entity_key);
        }
        event => panic!(
            "Expected EntityRemoved for key {} event, but got: {event:?}",
            entity.entity_key
        ),
    }
    Ok(())
}
