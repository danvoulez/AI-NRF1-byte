//! `ubl-capsule` v1 — Canon → ID → Signature → Hops
//!
//! Schema: `ubl-capsule/1.0`
//!
//! A capsule is the atomic unit of custody in UBL. It contains:
//!   - `domain`: always `"ubl-capsule/1.0"`
//!   - `hdr`: routing/metadata (src, dst, nonce, ts, act)
//!   - `env`: the payload (body, links, evidence)
//!   - `seal`: author signature over {domain, id, hdr, env}
//!   - `receipts`: append-only chain of SIRP hops
//!
//! The `id` is stable: it does NOT change when receipts/signatures are added.

pub mod id;
#[cfg(feature = "metrics")]
mod metrics_support;
pub mod receipt;
pub mod seal;
pub mod types;

pub use id::compute_id;
pub use types::*;
