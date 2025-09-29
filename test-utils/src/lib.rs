use anyhow::Result;
use arkiv_sdk::{ArkivClient, PrivateKeySigner};
use dirs::config_dir;
use url::Url;

/// Default URL for Arkiv node in tests
pub const ARKIV_URL: &str = "http://localhost:8545";
pub const ARKIV_WS_URL: &str = "ws://localhost:8545";

/// Default TTL value for test entities
pub const TEST_TTL: u64 = 30;

pub const TEST_KEYSTORE_PASSPHRASE: &str = "passphrase";

pub fn get_client() -> Result<ArkivClient> {
    let keypath = config_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?
        .join("golembase")
        .join("wallet.json");
    let signer = PrivateKeySigner::decrypt_keystore(keypath, TEST_KEYSTORE_PASSPHRASE)?;
    let url = Url::parse(ARKIV_URL)?;
    let client = ArkivClient::builder().wallet(signer).rpc_url(url).build();
    Ok(client)
}
