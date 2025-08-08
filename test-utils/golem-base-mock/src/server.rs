use alloy::primitives::{Address, U256};
use std::net::SocketAddr;
use std::str::FromStr;

use crate::GolemBaseMockServer;

/// Helper function to start a GolemBase mock server on a specific address
pub async fn start_mock_server(
    addr: &str,
) -> Result<GolemBaseMockServer, Box<dyn std::error::Error + Send + Sync>> {
    let socket_addr = SocketAddr::from_str(addr)?;
    let server = GolemBaseMockServer::new().start(socket_addr).await?;
    Ok(server)
}

/// Helper function to start a GolemBase mock server with default configuration
pub async fn start_default_mock_server(
) -> Result<GolemBaseMockServer, Box<dyn std::error::Error + Send + Sync>> {
    start_mock_server("127.0.0.1:8545").await
}

/// Helper function to create a mock server with test accounts and balances
pub async fn create_test_mock_server(
) -> Result<GolemBaseMockServer, Box<dyn std::error::Error + Send + Sync>> {
    // Create test accounts
    let test_accounts = vec![
        Address::from_str("0x70997970C51812dc3A010C7d01b50e0d17dc79C8")?,
        Address::from_str("0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC")?,
        Address::from_str("0x90F79bf6EB2c4f870365E785982E1f101E93b906")?,
    ];

    let server = GolemBaseMockServer::new()
        .with_chain_id(31337) // Hardhat default
        .with_accounts(test_accounts.clone())
        .with_syncing(false);

    // Start the server
    let socket_addr = SocketAddr::from_str("127.0.0.1:8545")?;
    let mut server = server.start(socket_addr).await?;

    // Set initial balances for test accounts
    for account in test_accounts {
        server
            .state_mut()
            .data
            .write()
            .await
            .balances
            .insert(account, U256::from(1000000000000000000000u128));
    }

    Ok(server)
}

/// Helper function to get the URL for the mock server
pub fn get_mock_server_url(addr: &str) -> String {
    format!("http://{}", addr)
}

/// Helper function to get the default mock server URL
pub fn get_default_mock_server_url() -> String {
    get_mock_server_url("127.0.0.1:8545")
}
