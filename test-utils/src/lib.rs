use anyhow::Result;
use dirs::config_dir;
use golem_base_sdk::{GolemBaseClient, PrivateKeySigner};
use url::Url;

/// Default URL for GolemBase node in tests
pub const GOLEM_BASE_URL: &str = "http://localhost:8545";
pub const GOLEM_BASE_WS_URL: &str = "ws://localhost:8545";

/// Default TTL value for test entities
pub const TEST_TTL: u64 = 30;

pub const TEST_KEYSTORE_PASSPHRASE: &str = "passphrase";

pub fn get_client() -> Result<GolemBaseClient> {
    let keypath = config_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?
        .join("golembase")
        .join("wallet.json");
    let signer = PrivateKeySigner::decrypt_keystore(keypath, TEST_KEYSTORE_PASSPHRASE)?;
    let url = Url::parse(GOLEM_BASE_URL)?;
    let client = GolemBaseClient::builder()
        .wallet(signer)
        .rpc_url(url)
        .build();
    Ok(client)
}
