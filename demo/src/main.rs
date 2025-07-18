use dirs::config_dir;
use futures::StreamExt;
use golem_base_sdk::entity::{Create, EntityResult, Extend, Update};
use golem_base_sdk::events::EventsClient;
use golem_base_sdk::{
    Address, Annotation, GolemBaseClient, GolemBaseRoClient, Hash, PrivateKeySigner, Url,
};
use log::info;
use std::fs;

async fn log_num_of_entities_owned(client: &GolemBaseRoClient, owner_address: Address) {
    let n = client
        .get_entities_of_owner(owner_address)
        .await
        .expect("Failed to fetch entities of owner")
        .len();
    info!("Number of entities owned: {}", n);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut private_key_path = config_dir().ok_or("Failed to get config directory")?;
    private_key_path.push("golembase/private.key");
    let private_key_bytes = fs::read(&private_key_path).map_err(|e| {
        format!(
            "Failed to read private key at {}: {}",
            private_key_path.display(),
            e
        )
    })?;
    let private_key = Hash::from_slice(&private_key_bytes);

    let signer = PrivateKeySigner::from_bytes(&private_key)
        .map_err(|e| format!("Failed to parse private key: {}", e))?;
    let url = Url::parse("http://localhost:8545").unwrap();
    let client = GolemBaseClient::builder()
        .wallet(signer)
        .rpc_url(url)
        .build();

    info!("Fetching owner address...");
    let owner_address = client.get_owner_address();
    info!("Owner address: {}", owner_address);
    log_num_of_entities_owned(&client, owner_address).await;

    tokio::spawn(async move {
        let events_client = EventsClient::new(Url::parse("ws://localhost:8545").unwrap())
            .await
            .unwrap();
        let mut event_stream = events_client.events_stream().await.unwrap();
        while let Some(event) = (event_stream).next().await {
            info!("Got event: {:?}", event)
        }
    });

    info!("Creating entities...");
    let creates = vec![
        Create {
            data: "foo".into(),
            btl: 25,
            string_annotations: vec![Annotation::new("key", "foo")],
            numeric_annotations: vec![Annotation::new("ix", 1u64)],
        },
        Create {
            data: "bar".into(),
            btl: 2,
            string_annotations: vec![Annotation::new("key", "bar")],
            numeric_annotations: vec![Annotation::new("ix", 2u64)],
        },
        Create {
            data: "qux".into(),
            btl: 50,
            string_annotations: vec![Annotation::new("key", "qux")],
            numeric_annotations: vec![Annotation::new("ix", 3u64)],
        },
    ];
    let receipts: Vec<EntityResult> = client.create_entities(creates).await?;
    info!("Created entities: {:?}", receipts);
    log_num_of_entities_owned(&client, owner_address).await;

    info!("Deleting first entity...");
    client.delete_entities(vec![receipts[0].entity_key]).await?;
    log_num_of_entities_owned(&client, owner_address).await;

    info!("Updating the third entity...");
    let third_entity_key = receipts[2].entity_key;
    let metadata = client.get_entity_metadata(third_entity_key).await?;
    info!("... before the update: {:?}", metadata);
    client
        .update_entities(vec![Update {
            data: "foobar".into(),
            btl: 40,
            string_annotations: vec![Annotation::new("key", "qux"), Annotation::new("foo", "bar")],
            numeric_annotations: vec![Annotation::new("ix", 2u64)],
            entity_key: third_entity_key,
        }])
        .await?;
    let metadata = client.get_entity_metadata(third_entity_key).await?;
    info!("... after the update: {:?}", metadata);

    info!("Extending the third entity...");
    let metadata = client.get_entity_metadata(third_entity_key).await?;
    info!("... before the extension: {:?}", metadata);
    client
        .extend_entities(vec![Extend {
            entity_key: third_entity_key,
            number_of_blocks: 60,
        }])
        .await?;
    let metadata = client.get_entity_metadata(third_entity_key).await?;
    info!("... after the extension: {:?}", metadata);

    info!("Deleting remaining entities...");
    let remaining_entities = client
        .query_entity_keys("ix = 1 || ix = 2 || ix = 3")
        .await?;
    info!("Remaining entities: {:?}", remaining_entities);
    client.delete_entities(remaining_entities).await?;
    log_num_of_entities_owned(&client, owner_address).await;

    Ok(())
}
