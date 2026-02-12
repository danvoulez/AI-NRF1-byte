# Constitution of the Base

*Ratified upon completion of the BASE phase. These laws are immutable.*
*Any change to a Base law requires a new major version of ai-nrf1.*

---

## Preamble

The Base is the terrain upon which all modules stand and all products are built.
It is not a product. It is not a feature. It is the ground truth.

The Base exists so that every act — past, present, and future — can be
reduced to a single deterministic chain of bytes, hashes, and signatures
that any party can verify without trust, without permission, and without
the original author being present.

---

## Article I — ρ (Rho): The Policy Engine at the Byte Level

**Before any hash is computed, before any signature is applied, ρ runs.**

ρ is not a pre-processing step. ρ is not a convenience function.
ρ is the policy engine that operates at the lowest possible level — the byte
level — ensuring that every value has exactly ONE canonical form before it
enters the cryptographic pipeline.

**Crate:** `impl/rust/nrf-core/src/rho.rs`

### Section 1.1 — The ρ Rules

| # | Rule | What ρ does |
|---|------|-------------|
| 1 | **Strings** | UTF-8 validated, NFC normalized, BOM (U+FEFF) rejected |
| 2 | **Timestamps** | RFC-3339 UTC 'Z', minimal fractional seconds, no `.000` |
| 3 | **Decimals** | No exponent, no leading zeros, no superfluous `.0`, `-0` → `0` |
| 4 | **Sets** | Sorted by canonical NRF bytes of each element, deduplicated |
| 5 | **Maps** | Keys sorted by UTF-8 bytes (BTreeMap), **null values removed** (absence ≠ null) |
| 6 | **Recursion** | ρ normalizes leaves first, then containers |
| 7 | **Passthrough** | Null, Bool, Int64, Bytes are already canonical — unchanged |

### Section 1.2 — The ρ Properties

These are mathematical guarantees, not aspirations:

```
ρ(ρ(v)) = ρ(v)                              — idempotent
encode(ρ(v)) = encode(ρ(v))                  — deterministic
encode(ρ(v)) is the ONLY valid encoding of v — canonical
BLAKE3(encode(ρ(v))) is the ONLY valid hash  — hash-stable
```

### Section 1.3 — The ρ Modes

ρ operates in two modes:

- **Normalize** (`rho::normalize`) — Rewrites a sloppy value into canonical form.
  Use this when accepting input from external sources (APIs, LLMs, users).
- **Validate** (`rho::validate`) — Checks that a value is ALREADY canonical.
  Returns an error if ρ would change anything. Use this when verifying artifacts.

### Section 1.4 — The Blessed Path

The only valid path from value to CID is through ρ:

```
Value → ρ(Value) → encode → BLAKE3 → CID (b3:<hex>) → Ed25519 sig → Rich URL
```

Any code that calls `encode()` without first calling `rho::normalize()` or
`rho::validate()` is producing potentially non-canonical bytes. This is a
constitutional violation.

The convenience functions `rho::canonical_encode()` and `rho::canonical_cid()`
enforce this path. They are the recommended API.

---

## Article II — The Fractal Invariant

**One value. One byte stream. One hash. One signature. One decision.**

At every layer of the system, the same transformation applies:

```
ρ(Value) → NRF bytes → BLAKE3 hash → CID (b3:<hex>) → Ed25519 signature → Rich URL
```

This is not a guideline. This is the law. Any crate, module, or product
that breaks this chain is unconstitutional and must be corrected or removed.

### Section 2.1 — Canonicality

There exists exactly ONE valid ai-nrf1 byte encoding for any given logical value.
That encoding is `encode(ρ(value))`. No other byte stream is valid for that value.
Two implementations that produce different bytes for the same value are in violation.

### Section 2.2 — Determinism

Given the same input, the same key, and the same timestamp, the system MUST
produce the same receipt, the same CID, and the same signature. Every time.
On every machine. Forever. ρ guarantees this at the value level. The encoder
guarantees this at the byte level. Together they are absolute.

### Section 2.3 — Verifiability

Any artifact produced by the system (receipt, permit, ghost, capsule) MUST be
independently verifiable using only: the artifact itself, the signer's public key,
and the ai-nrf1 specification (which includes ρ). No database. No API call. No trust.

---

## Article III — The Seven Types

ai-nrf1 recognizes exactly seven value types:

| Tag | Type   | Encoding |
|-----|--------|----------|
| 0   | Null   | tag only |
| 1   | Bool   | tag + 1 byte |
| 2   | Int64  | tag + 8 bytes big-endian |
| 3   | String | tag + varint32 length + UTF-8 NFC bytes |
| 4   | Bytes  | tag + varint32 length + raw bytes |
| 5   | Array  | tag + varint32 count + elements |
| 6   | Map    | tag + varint32 count + sorted key-value pairs |

### Section 3.1 — No Floats

There are no floating-point numbers. If you need decimals, use fixed-point
integers with a known scale. This is not a limitation. This is a guarantee
that `encode(decode(bytes)) == bytes` for all time.

### Section 3.2 — Sorted Maps

Map keys are sorted by raw UTF-8 byte comparison. Not locale. Not Unicode
collation. Raw bytes. This ensures canonical encoding without configuration.

### Section 3.3 — NFC Strings

All strings MUST be Unicode NFC normalized by ρ before encoding. ρ normalizes;
the decoder rejects non-NFC. This is the dual enforcement: ρ fixes on the way in,
the decoder rejects on the way out.

---

## Article IV — The Canonical Artifacts

The Base defines exactly four canonical artifacts. Each follows the fractal invariant.

### Section 4.1 — Receipt

The Receipt is THE artifact. Every act in the system produces one.
It binds: identity, time, act, decision, body, runtime attestation,
chain position, and signature into a single canonical object.

**Crate:** `crates/receipt`

**Invariants:**
- `receipt_cid = b3(NRF(receipt_without_sig))`
- `body_cid = b3(NRF(body))`
- `sig = Ed25519(BLAKE3(NRF(receipt_without_sig)))`
- GHO-001: If `decision == "GHOST"` then `effects` MUST be `None`

### Section 4.2 — Permit

The Permit is the accountability closer. It is a signed, time-bounded,
hash-pinned authorization from an authority. Without a permit, "ALLOW"
is a claim. With a permit, "ALLOW" is math.

**Crate:** `crates/permit`

**Invariants:**
- `permit_cid = b3(NRF(permit_without_sig))`
- `decision` MUST be `"ALLOW"` (only ALLOW gets a permit)
- `expires_at > issued_at`
- `input_hash` MUST match the actual input at verification time

### Section 4.3 — Ghost

The Ghost is the Write-Before-Execute record. Every execution MUST begin
as a Ghost(pending). The ghost is immutable evidence of intent.

**Crate:** `crates/ghost`

**State machine:**
```
WBE ──create──▶ GHOST[pending]
GHOST[pending] ──promote──▶ links to final RECEIPT
GHOST[pending] ──expire──▶  GHOST[expired] (cause recorded)
```

**Invariants:**
- `ghost_cid = b3(NRF(ghost_without_sig))`
- Pending ghosts MUST NOT have a `cause`
- Expired ghosts MUST have a `cause` (timeout | canceled | drift)

### Section 4.4 — Capsule

The Capsule is the wire format. It wraps NRF bytes with sender identity,
role, signature, and a CID.

**Crate:** `crates/ubl-transport`

**Invariants:**
- `payload_cid = b3(payload)`
- `sig = Ed25519(BLAKE3(header_bytes || payload))`

---

## Article V — The Three Acts

Every interaction in the system is one of exactly three acts:

| Act | Meaning |
|-----|---------|
| **ATTEST** | "This thing exists and has these properties" |
| **EVALUATE** | "Given these rules, this passes or fails" |
| **TRANSACT** | "Two parties exchanged something, here's proof" |

**Crate:** `crates/acts`

### Section 5.1 — Completeness

These three acts are complete. Every product is a composition of them.
There is no fourth act. If a new verb cannot be expressed as ATTEST,
EVALUATE, or TRANSACT, the verb is wrong, not the taxonomy.

### Section 5.2 — Composition via pipeline_prev

Any act can reference the CIDs of prior act receipts via `pipeline_prev`.
This enables free composition without a workflow engine.

```
Insurance = ATTEST → EVALUATE
Credit    = ATTEST → EVALUATE
Auditing  = EVALUATE over past receipts
```

---

## Article VI — The Four Decisions

The policy gate produces exactly one of four decisions:

| Decision | Meaning |
|----------|---------|
| **ALLOW** | Proceed. Effects will be executed. |
| **DENY** | Rejected. No effects. Reason recorded. |
| **REQUIRE** | Needs consent (k-of-n) before proceeding. |
| **GHOST** | Write-Before-Execute. Recorded but not effectuated. |

**Crate:** `crates/ubl-policy`

---

## Article VII — The Five Policy Families

Every policy engine declares which family it belongs to:

| Family | Question it answers |
|--------|-------------------|
| **Existence** | Is it well-formed? Required fields present? |
| **Compliance** | Does it meet regulatory rules? |
| **Threshold** | Is a numeric value within bounds? |
| **Provenance** | Was it built/signed/attested correctly? |
| **Authorization** | Can this actor do this thing to this resource? |

These are metadata for routing. The engine does not branch on family.
Modules implement families. The Base defines the socket.

---

## Article VIII — The Certified Runtime

**Every Receipt carries proof of WHERE and HOW it was produced.**

The Certified Runtime is the execution environment that produces a Receipt.
Its identity — binary hash, version, environment, certificates — enters the
Receipt hash preimage through ρ-normalized `RuntimeInfo`.

**Crate:** `crates/runtime`

### Section 8.1 — The RuntimeInfo Contract

Every Receipt MUST contain a valid `RuntimeInfo` with:

| Field | Requirement |
|-------|-------------|
| `name` | Non-empty. Identifies the runtime implementation. |
| `version` | Non-empty. Semantic version of the runtime. |
| `binary_sha256` | Non-empty. SHA-256 of the running binary. |
| `hal_ref` | Optional. Hardware abstraction layer reference. |
| `env` | Map of environment variables relevant to reproducibility. |
| `certs` | Hardware attestation certificates (if available). No empty certs. |

### Section 8.2 — ρ-Level Guarantees

`RuntimeInfo` is ρ-validated before entering the Receipt:

```
RuntimeInfo → to_canonical_value() → ρ(Value) → encode → BLAKE3 → rt_cid
```

Same runtime + same environment → same bytes → same CID. Always.
This is the byte-level identity of the execution environment.

### Section 8.3 — The RuntimeAttestation Trait

The BASE defines the **interface**. Modules provide the **implementation**.

```rust
pub trait RuntimeAttestation: Send + Sync {
    fn attest(&self, req: &AttestationRequest) -> Result<AttestationResponse, RuntimeError>;
    fn runtime_name(&self) -> &str;
    fn runtime_version(&self) -> &str;
}
```

The contract:
1. `attest()` MUST return a `RuntimeInfo` that passes `validate()`
2. Same input → same `RuntimeInfo` → same CID (determinism)
3. `binary_sha256` MUST be the actual SHA-256 of the running binary
4. `certs` SHOULD include hardware attestation certificates when available

### Section 8.4 — Built-in: SelfAttestation

The BASE ships `SelfAttestation` — a runtime that reports its own binary hash
and environment. This is the development/testing runtime. It is honest but
not hardware-attested. Production deployments SHOULD use a TEE or
deterministic runtime module.

### Section 8.5 — Module Implementations

These are MODULES, not BASE:

| Module | What it attests |
|--------|----------------|
| `runtime-llm` | Deterministic LLM (seed=0, temp=0, top_p=1) |
| `runtime-wasm` | Sandboxed WASM with memory limits |
| `runtime-tee` | Hardware TEE (SGX, TrustZone, Nitro Enclaves) |

---

## Article IX — The Three Proof Levels

Every interaction can be configured to one of three proof levels:

| Level | What's produced |
|-------|----------------|
| **Receipt** | Engine receipt only. Cheapest. Hash chain + signature. |
| **SIRP** | INTENT + RESULT capsules. Delivery + execution receipts. |
| **Bundle** | Everything above + offline-verifiable ZIP with manifest. |

The proof level is a configuration flag, not a code path.

---

## Article X — Non-Negotiable Principles

### Section 10.1 — No Hidden State

Every state transition MUST emit a receipt. If it didn't produce a receipt,
it didn't happen.

### Section 10.2 — No Trusted Agents

No agent, service, or runtime is trusted by default. Trust is established
by verifying signatures against known public keys. Always.

### Section 10.3 — No Convenience Abstractions That Hide Security Cost

If an abstraction makes it easier to skip verification, the abstraction
is unconstitutional. Convenience must never come at the cost of verifiability.

### Section 10.4 — No Provider Lock-in

Every artifact is self-contained and verifiable offline. Moving from one
provider to another requires zero data transformation. The bytes are the bytes.

---

## Article XI — The Crate Law

### Section 11.1 — Single Source of Truth

`impl/rust/nrf-core` is the ONE canonical implementation of ai-nrf1
encode/decode. All other crates delegate to it. There are no alternative
implementations in the workspace.

### Section 11.2 — Every Crate Compiles

`cargo check --workspace` MUST return zero errors at all times.
A crate that does not compile is a crate that does not exist.

### Section 11.3 — No Orphans

Every crate in the workspace MUST be listed in the root `Cargo.toml` members.
A crate outside the workspace is invisible and therefore dangerous.

---

## Signatures

```
Ratified: BASE Phase Complete
Tests:    105 pass (base-conformance: 63, nrf-core+rho: 23, runtime: 10, receipt-gateway: 9)
Workspace: cargo check --workspace = 0 errors
Articles: 11 (I=ρ, II=Fractal, III=Types, IV=Artifacts, V=Acts, VI=Decisions,
          VII=Families, VIII=Runtime, IX=Proofs, X=Principles, XI=Crates)
Crates:   nrf-core, nrf1, ai-nrf1, receipt, permit, ghost, acts, runtime,
          reasoning-bit, sdk-rs, envelope, ubl-json, ubl-transport,
          ubl-policy, ubl-auth, ubl-model, ubl-storage, signers,
          registry, nrf1-cli, cli/nrf1
Modules:  receipt-gateway
```
