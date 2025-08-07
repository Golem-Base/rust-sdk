//! GolemBase container testing utilities.
//!
//! This module provides utilities for running GolemBase in containers for testing purposes.

use std::time::Duration;
use testcontainers::core::logs::LogFrame;
use testcontainers::core::{ContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};
use url::Url;

/// Configuration for GolemBase container.
pub struct Config {
    /// Port for the GolemBase instance
    pub port: u16,
    /// Timeout for waiting for container to start
    pub timeout: Duration,
    /// Container image to use
    pub image: String,
    /// Container tag to use
    pub tag: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 9545,
            timeout: Duration::from_secs(120), // Increased timeout for stability
            image: "quay.io/golemnetwork/gb-op-geth".to_string(),
            tag: "latest".to_string(),
        }
    }
}

impl Config {
    /// Set the port for the GolemBase instance
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the timeout for container operations
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Wrapper for GolemBase container that provides helper functions.
pub struct GolemBaseContainer {
    container: ContainerAsync<GenericImage>,
    config: Config,
    mapped_port: u16,
}

impl GolemBaseContainer {
    /// Initialize a new GolemBase container with the given configuration.
    pub async fn new(config: Config) -> Result<Self, anyhow::Error> {
        let container = Self::init_golembase(&config).await?;
        let mapped_port = container.get_host_port_ipv4(config.port).await?;
        Ok(Self {
            container,
            config,
            mapped_port,
        })
    }

    /// Get the container URL that can be used with GolemBaseClient.
    pub fn get_url(&self) -> Result<Url, anyhow::Error> {
        Ok(Url::parse(&format!(
            "http://localhost:{}",
            self.mapped_port
        ))?)
    }

    /// Get the container ID for debugging purposes.
    pub fn container_id(&self) -> String {
        self.container.id().to_string()
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Initialize the GolemBase container with the given configuration.
    async fn init_golembase(
        config: &Config,
    ) -> Result<ContainerAsync<GenericImage>, anyhow::Error> {
        let port = config.port;
        let timeout = config.timeout;

        let container_future = GenericImage::new(&config.image, &config.tag)
            .with_wait_for(WaitFor::message_on_stderr("HTTP server started"))
            .with_mapped_port(port, ContainerPort::Tcp(port))
            .with_log_consumer(|line: &LogFrame| {
                log::info!("[GolemBase]: {}", String::from_utf8_lossy(&line.bytes()))
            })
            .with_cmd([
                "--dev",
                "--http",
                "--http.api",
                "eth,web3,net,debug,golembase",
                "--verbosity",
                "3",
                "--http.addr",
                "0.0.0.0",
                "--http.port",
                &port.to_string(),
                "--http.corsdomain",
                "*",
                "--http.vhosts",
                "*",
                "--ws",
                "--ws.addr",
                "0.0.0.0",
                "--ws.port",
                &port.to_string(),
            ])
            .with_env_var("GITHUB_ACTIONS", "true")
            .with_env_var("CI", "true")
            .start();

        let container = match tokio::time::timeout(timeout, container_future).await {
            Ok(Ok(container)) => container,
            Ok(Err(e)) => return Err(anyhow::anyhow!("Failed to start GolemBase instance: {}", e)),
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Timeout ({}) starting GolemBase instance",
                    humantime::format_duration(timeout)
                ))
            }
        };

        Ok(container)
    }
}
