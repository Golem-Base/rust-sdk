use anyhow::Result;
use bigdecimal::BigDecimal;
use golem_base_sdk::{client::GolemBaseClient, entity::Create, Url};
use serial_test::serial;

const GOLEM_BASE_URL: &str = "http://localhost:8545";

fn init_logger(should_init: bool) {
    if should_init {
        let _ = env_logger::try_init();
    }
}

#[tokio::test]
#[serial]
async fn test_query_entities() -> Result<()> {
    init_logger(false);
    let client = GolemBaseClient::new(Url::parse(GOLEM_BASE_URL)?)?;

    // Create test account
    let account = client.account_generate("test123").await?;
    let fund_tx = client.fund(account, BigDecimal::from(1)).await?;
    log::info!("Created and funded account: {account}, tx: {fund_tx}");

    // Cleanup - remove all existing entities
    let all_entity_keys = client.get_all_entity_keys().await?;
    log::info!("Removing all existing entities: {:?}", all_entity_keys);
    if !all_entity_keys.is_empty() {
        client.remove_entries(account, all_entity_keys).await?;
    }

    // Create entries with different annotations
    let entry1 = Create::new(b"test1".to_vec(), 1000)
        .annotate_string("type", "test")
        .annotate_string("category", "alpha");
    let entry1_id = client.create_entry(account, entry1).await?;
    log::info!("Created entry1: {entry1_id}");

    let entry2 = Create::new(b"test2".to_vec(), 1000)
        .annotate_string("type", "test")
        .annotate_string("category", "beta");
    let entry2_id = client.create_entry(account, entry2).await?;
    log::info!("Created entry2: {entry2_id}");

    let entry3 = Create::new(b"test3".to_vec(), 1000)
        .annotate_string("type", "demo")
        .annotate_string("category", "alpha");
    let entry3_id = client.create_entry(account, entry3).await?;
    log::info!("Created entry3: {entry3_id}");

    // Test queries
    let type_test_entries = client.query_entity_keys("type = \"test\"").await?;
    log::info!("Entries with type = \"test\": {:?}", type_test_entries);
    assert_eq!(type_test_entries.len(), 2);
    assert!(type_test_entries.contains(&entry1_id));
    assert!(type_test_entries.contains(&entry2_id));

    let category_alpha_entries = client.query_entity_keys("category = \"alpha\"").await?;
    log::info!(
        "Entries with category = \"alpha\": {:?}",
        category_alpha_entries
    );
    assert_eq!(category_alpha_entries.len(), 2);
    assert!(category_alpha_entries.contains(&entry1_id));
    assert!(category_alpha_entries.contains(&entry3_id));

    let type_demo_entries = client.query_entity_keys("type = \"demo\"").await?;
    log::info!("Entries with type = \"demo\": {:?}", type_demo_entries);
    assert_eq!(type_demo_entries.len(), 1);
    assert!(type_demo_entries.contains(&entry3_id));

    let combined_and = client
        .query_entity_keys("type = \"test\" && category = \"beta\"")
        .await?;
    log::info!(
        "Entries with type = \"test\" && category = \"beta\": {:?}",
        combined_and
    );
    assert_eq!(combined_and.len(), 1);
    assert!(combined_and.contains(&entry2_id));

    let combined_or = client
        .query_entity_keys("type = \"demo\" || category = \"beta\"")
        .await?;
    log::info!(
        "Entries with type = \"demo\" || category = \"beta\": {:?}",
        combined_or
    );
    assert_eq!(combined_or.len(), 2);
    assert!(combined_or.contains(&entry2_id));
    assert!(combined_or.contains(&entry3_id));

    // Test empty result
    let no_results = client.query_entity_keys("type = \"nonexistent\"").await?;
    log::info!("Entries with type = \"nonexistent\": {:?}", no_results);
    assert_eq!(no_results.len(), 0);

    // Test selecting all entries
    let all_entries = client
        .query_entity_keys("type = \"test\" || type = \"demo\"")
        .await?;
    log::info!("All entries: {:?}", all_entries);
    assert_eq!(all_entries.len(), 3);
    assert!(all_entries.contains(&entry1_id));
    assert!(all_entries.contains(&entry2_id));
    assert!(all_entries.contains(&entry3_id));

    Ok(())
}
