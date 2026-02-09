//! `ai-nrf1` — canonical ai-nrf1 binary encoding crate.
//!
//! Use `ai-nrf1` / `ai-json-nrf1` when the **norm** matters.
//! Use `ubl-byte` / `ubl-json` when the **brand** matters.
//! Both map to the same canon.
//!
//! All encoding/decoding delegates to `nrf_core` so the invariant
//! **one value => one byte stream => one hash** holds everywhere.

pub use nrf_core::Value;
pub use nrf_core::Error;
pub use nrf_core::Result;
pub use nrf_core::MAGIC;

pub use nrf_core::encode;
pub use nrf_core::decode;
pub use nrf_core::encode_stream;
pub use nrf_core::blake3_cid;
pub use nrf_core::hash_bytes;
pub use nrf_core::hash_value;

#[cfg(feature = "compat_cbor")]
pub mod compat_cbor;
