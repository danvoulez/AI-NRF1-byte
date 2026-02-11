//! Module-phase runtime orchestrator.
//!
//! Reads a product manifest, iterates pipeline steps, calls capabilities,
//! collects hop receipts, and executes effects.
//!
//! This is NOT `crates/runtime` (which holds BASE primitives like attestation).
//! See `docs/MODULES-DESIGN.md` sections 3–4, 10 for rationale.

pub mod adapters;
pub mod assets;
pub mod bindings;
pub mod cap_registry;
pub mod effects;
pub mod errors;
pub mod manifest;
pub mod runner;
