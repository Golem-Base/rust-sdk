use alloy::primitives::U256;
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
    // Create server
    let mut server = GolemBaseMockServer::new().with_chain_id(31337); // Hardhat default

    // Create test accounts with initial balances
    for _ in 0..3 {
        server
            .create_test_account(U256::from(1000000000000000000000u128))
            .await;
    }

    // Start the server
    let socket_addr = SocketAddr::from_str("127.0.0.1:8545")?;
    let server = server.start(socket_addr).await?;

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
