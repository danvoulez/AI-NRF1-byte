//! Thin facade over `nrf-core` — the single canonical NRF-1.1 implementation.
//! All types and functions are re-exported from `nrf_core`.

pub use nrf_core::Value;
pub use nrf_core::Error as NrfError;
pub use nrf_core::Result;
pub use nrf_core::MAGIC;

pub use nrf_core::encode;
pub use nrf_core::decode;
pub use nrf_core::hash_bytes;
pub use nrf_core::hash_value;

// Compat aliases used by receipt, reasoning-bit, sdk-rs
pub use nrf_core::encode_stream;
pub use nrf_core::blake3_cid;

/// Compat alias: old crates/nrf1 called this `decode_stream`.
pub fn decode_stream(bytes: &[u8]) -> Result<Value> {
    decode(bytes)
}
