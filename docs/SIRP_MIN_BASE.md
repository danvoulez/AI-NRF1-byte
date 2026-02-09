
# SIRP Compatibility (Minimal for BASE)

> **Terminologia oficial (estável):** **ai-nrf1** (bytes) e **ai-json-nrf1** (view).
> **Aliases de marca (equivalentes):** **ubl-byte** (bytes) e **ubl-json** (view).

- We keep HTTP as the transport but **adopt SIRP capsule semantics**:
  - `capsule.payload = ai-nrf1 bytes` (or JSON✯Atomic bytes)
  - `capsule.cid = b3(payload)`
  - `capsule.sig = Ed25519(domain="sirp.cap.v1", header||payload)`
- For BASE:
  - Emit **delivery receipts** in the registry with `{ capsule_cid, outcome }`.
  - Add `sirp.receipt.delivery.v1` schema later; for now a field in receipt:`network: { capsule_cid, delivered: true }`.