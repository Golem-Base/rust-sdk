// Re-export commonly used types from `alloy`.
pub use alloy::primitives::{keccak256, Address};
pub use alloy::signers::local::PrivateKeySigner;
pub use alloy::signers::Signature;
pub use alloy::transports::http::reqwest::Url;

pub use client::GolemBaseClient;
pub use entity::{Annotation, Hash, NumericAnnotation, StringAnnotation};

/// Module for Ethereum transaction-related functionality.
pub mod eth;

/// Module for JSON-RPC-related functionality.
pub mod rpc;

/// Module for GolemBase client functionality.
pub mod client;

pub mod account;
pub mod entity;
pub mod events;
pub mod signers;
pub mod utils;
