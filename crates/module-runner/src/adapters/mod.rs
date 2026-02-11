//! Concrete effect adapters — gated behind the `live` feature.
//!
//! These provide real HTTP, signing, LLM, and durable idempotency
//! implementations that plug into `DispatchExecutor`.

pub mod http;
pub mod idem;
pub mod llm;
pub mod permit;
pub mod permit_http;
pub mod resume;
pub mod signer;
