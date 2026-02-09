# Deferred Hardening Issues

Tracked items from the canonical audit that require API changes or cross-crate refactoring.

## Issue 2: Floats leaking from JSON layer

**Crates:** `crates/ubl-json`, `crates/reasoning-bit`
**Problem:** `UblJsonV1.confidence` is `Option<f64>`; `reasoning-bit` uses `f32` for `confidence`, `hrd_score`, `temperature`, `top_p`.
**Fix:** Replace all `f32`/`f64` fields with canonical decimal strings or scaled `i64` (micro-units). The `to_nrf()` methods already do `(x * 1_000_000.0) as i64` — make the struct fields match.
**Risk:** API-breaking change for consumers of these structs.

## Issue 7: DID/KID ASCII+NFC enforcement in registry

**Crates:** `services/registry`
**Problem:** `validate_ascii()` and `validate_nfc()` exist in `nrf-core` and `ubl_json_view` but are not called on DID/KID fields in registry routes (`issuer_did`, `subject`, etc.).
**Fix:** Add validation calls in `create_receipt` for all DID/KID string fields before processing. Reject non-ASCII DIDs at the HTTP boundary.
**Risk:** Low — additive validation only.

## Issue 9: AWS SDK behavior-version

**Crates:** `crates/ubl-storage`
**Problem:** `aws_config::load_defaults(BehaviorVersion::latest())` uses the latest behavior version at compile time. Should pin to a specific version for reproducibility.
**Fix:** Pin to `BehaviorVersion::v2024_03_28()` (or latest stable) and document the choice.
**Risk:** None — cosmetic/reproducibility improvement.

## Issue 11: reasoning-bit serialization boundary

**Crates:** `crates/reasoning-bit`
**Problem:** `ReasoningBit` derives `Serialize`/`Deserialize` with no gate. The `to_nrf()` method does float-to-int conversion inline without passing through rho normalization.
**Fix:** Remove `Serialize`/`Deserialize` derives (or gate behind a feature). Add `to_canonical_value()` that goes through `rho::normalize()`. Make `sign_bytes()` use `rho::canonical_encode()`.
**Risk:** API-breaking for anything that serializes `ReasoningBit` directly to JSON.
