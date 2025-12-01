use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

/// The blocks-to-live (BTL) for the entity.
///
/// # Example
///
/// Each block is roughly 2 seconds of life. The following shows
/// one way to construct this in an ergonomic and reusable way
/// that also takes advantage of compile time checks.
///
/// ```rs
/// use arkiv_sdk::entity::BlocksToLive;
///
/// const THIRTY_SECONDS: BlocksToLive = BlocksToLive::new(15u64);
/// ```
///
/// # Panics
///
/// Panics if the value is `u64::MIN`, i.e. it must be non-zero.
#[derive(Debug, Clone, Copy, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct BlocksToLive(u64);
impl BlocksToLive {
    pub const fn new(btl: u64) -> Self {
        if btl == u64::MIN {
            panic!("`BlocksToLive` must be non-zero");
        }
        Self(btl)
    }
}
impl Default for BlocksToLive {
    // We set this to 30s of life by default since we cannot have
    // a zero value, and anything less would be too short to be sane.
    fn default() -> Self {
        Self::new(15u64)
    }
}
impl From<u64> for BlocksToLive {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[test]
fn btl_const_compiles() {
    const THIRTY_SECONDS: BlocksToLive = BlocksToLive::new(15u64);
}
