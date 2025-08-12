use alloy::primitives::B256;
use alloy::signers::local::LocalSigner;
use anyhow::Result;
use dirs::config_dir;
use golem_base_sdk::{GolemBaseClient, PrivateKeySigner};
use std::fs;
use url::Url;

/// Default URL for GolemBase node in tests
pub const GOLEM_BASE_URL: &str = "http://localhost:8545";
pub const GOLEM_BASE_WS_URL: &str = "ws://localhost:8545";

/// Default TTL value for test entities
pub const TEST_TTL: u64 = 30;

pub fn get_client() -> Result<GolemBaseClient> {
    let keypath = config_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?
        .join("golembase")
        .join("wallet.json");
    let signer = LocalSigner::decrypt_keystore(keypath, "get password from stdin")?;
    let private_key_bytes = fs::read(&private_key_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read private key at {}: {}",
            private_key_path.display(),
            e
        )
    })?;
    let private_key = B256::from_slice(&private_key_bytes);

    let signer = PrivateKeySigner::from_bytes(&private_key)
        .map_err(|e| anyhow::anyhow!("Failed to parse private key: {}", e))?;
    let url = Url::parse(GOLEM_BASE_URL)?;
    let client = GolemBaseClient::builder()
        .wallet(signer)
        .rpc_url(url)
        .build();
    Ok(client)
}
