# ERRORS — Central Error Station

> The fractal applies to errors too.
> Every error follows one shape: `code + message + hint`.
> The code follows the layer taxonomy: `Err.<Layer>.<Detail>`.

## The Shape

```json
{
  "ok": false,
  "error": {
    "code": "Err.NRF.InvalidMagic",
    "message": "expected 'nrf1' magic header, got [0x00, 0x00, 0x00, 0x00]",
    "hint": "Ensure the buffer starts with the 4-byte NRF magic: 0x6e726631",
    "status": 400
  }
}
```

Every error tells you **what went wrong** (message) and **what to do** (hint).

## Implementation

Central crate: `crates/ubl-error/`

Every crate's error enum converts to `UblError` via `From` impls.
No crate had to change its error types — the central station adapts them all.

## Error Taxonomy

| Prefix | Layer | Source | Variants |
|--------|-------|--------|----------|
| Err.NRF.* | BASE | nrf-core | 17 |
| Err.Rho.* | BASE | nrf-core::rho | 3 |
| Err.JsonView.* | BASE | ubl_json_view | 13 |
| Err.Hop.* | BASE | ubl_capsule::receipt | 5 |
| Err.Seal.* | BASE | ubl_capsule::seal | 6 |
| Err.Runtime.* | BASE | runtime | 7 |
| Err.Ledger.* | BASE | ubl-storage | 2 |
| Err.Replay.* | BASE | ubl-replay | 3 |
| Err.Auth.* | BASE | ubl-auth | 4 |
| Err.Policy.* | MODULES | module-runner | 2 |
| Err.Permit.* | MODULES | module-runner | 3 |
| Err.Config.* | MODULES | module-runner | 2 |
| Err.IO.* | MODULES | module-runner | 4 |
| Err.UblJson.* | MODULES | ubl-json | 1 |

## Documents

| Doc | Topic |
|-----|-------|
| [LLM-FIRST-AUDIT.md](LLM-FIRST-AUDIT.md) | Full audit: findings, fixes, status |
| [ERROR-CODES.md](ERROR-CODES.md) | Error code reference |
