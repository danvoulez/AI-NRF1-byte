# Capsule Vectors (Signed)

This folder contains **committed, signed** capsule vectors for reproducible auditing.

The generation scripts use the repo's own CLI (`ubl`) to:

1. Sign the capsule JSON (`ubl cap sign`).
2. Encode the signed capsule into canonical **ai-nrf1** bytes (`ubl cap to-nrf`).

The private key is **never** committed. Only the resulting signed vectors and the public key used
for verification are committed.

## Files

- `capsule_ack.json` / `capsule_ack.nrf` / `capsule_ack.signed.json` / `capsule_ack.signed.nrf`
- `capsule_ask.json` / `capsule_ask.nrf` / `capsule_ask.signed.json` / `capsule_ask.signed.nrf`
- `capsule_nack.json` / `capsule_nack.nrf` / `capsule_nack.signed.json` / `capsule_nack.signed.nrf`
- `alice.pk` — hex-encoded Ed25519 public key (32 bytes) used to verify the above.

## Regeneration

From repo root:

```bash
make vectors
```

## Verification

```bash
make vectors-verify
```

