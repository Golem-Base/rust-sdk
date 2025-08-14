#![doc = include_str!("../README.md")]

/// Re-export commonly used types from `alloy`.
pub use alloy::{
    primitives::{Address, keccak256},
    signers::{Signature, local::PrivateKeySigner},
    transports::http::reqwest::Url,
};

pub use client::{GolemBaseClient, GolemBaseRoClient};
pub use entity::{Annotation, Hash, NumericAnnotation, StringAnnotation};

/// Module for Ethereum transaction-related functionality.
/// Provides helpers for constructing, signing, and sending Ethereum transactions.
pub mod eth;

/// Module for JSON-RPC-related functionality.
/// Contains utilities for interacting with JSON-RPC endpoints, including request/response types.
pub mod rpc;

/// Module for GolemBase client functionality.
/// Exposes the main client interface for interacting with the GolemBase network.
pub mod client;

/// Module for GolemBase entities and data types.
/// Defines core types such as annotations, hashes, and entity representations.
pub mod entity;

/// Module for event handling.
/// Contains types and utilities for working with GolemBase events.
pub mod events;

/// Module with utility functions.
/// Includes helpers for encoding, decoding, and other common tasks.
pub mod utils;
