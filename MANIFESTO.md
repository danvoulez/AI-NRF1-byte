# The ai-nrf1 Manifesto

*A canonical encoding for a world that cannot afford ambiguity.*

---

## I. The Problem

Every day, billions of decisions are made by software — approvals, denials,
evaluations, transactions, attestations. And every day, the same question
goes unanswered:

**"Can you prove that happened?"**

Not "can you show me a log." Not "can you query a database."
Can you hand me a single artifact that I can verify — right now,
on my machine, with no network, no API key, no trust in you —
and know with mathematical certainty that this decision was made,
by this entity, at this time, over this exact input?

Today, the answer is almost always no.

We built ai-nrf1 to make the answer always yes.

---

## II. The Insight

The problem is not cryptography. We have Ed25519. We have BLAKE3.
We have hash chains and Merkle trees and zero-knowledge proofs.

The problem is **canonicality**.

If two implementations encode the same logical value into different byte
streams, they produce different hashes. Different hashes mean different
signatures. Different signatures mean different "truths." And when two
parties disagree on what the bytes should be, no amount of cryptography
can reconcile them.

**One value must produce one byte stream. Always. Everywhere. Forever.**

This is the insight that ai-nrf1 is built on. Not a new cipher.
Not a new consensus algorithm. A new commitment to determinism
at the encoding layer — the one layer that everything else depends on.

---

## III. The Fractal

We discovered that the same pattern repeats at every scale:

```
Value → NRF bytes → BLAKE3 hash → CID → Ed25519 signature → Rich URL
```

A field in a receipt follows this pattern.
The receipt itself follows this pattern.
A capsule wrapping the receipt follows this pattern.
A bundle containing the capsule follows this pattern.

It is fractal. It is self-similar. It is the same six steps,
whether you are hashing a string or certifying an AI model.

We did not design this. We discovered it. And once we saw it,
we could not unsee it. Every architectural decision since has been
about protecting this fractal from corruption.

---

## IV. The Architecture

### Three Layers

```
┌─────────────────────────────────────────────┐
│                 PRODUCTS                     │
│  Manifests. Configuration. JSON. No code.   │
│  "What to build."                           │
├─────────────────────────────────────────────┤
│                 MODULES                      │
│  Policy engines. Intake adapters.           │
│  Enrichments. Runtimes.                     │
│  "How to build it."                         │
├─────────────────────────────────────────────┤
│                   BASE                       │
│  nrf-core · receipt · permit · ghost        │
│  acts · policy · transport · signers        │
│  "The ground truth."                        │
└─────────────────────────────────────────────┘
```

Each layer has its own constitution. Each constitution builds on the one
below it and never contradicts it.

### Three Acts

Every interaction in the system is one of three acts:

- **ATTEST** — "this thing exists and has these properties"
- **EVALUATE** — "given these rules, this passes or fails"
- **TRANSACT** — "two parties exchanged something, here's proof"

These are complete. Every product is a composition of them.
Insurance is ATTEST → EVALUATE. Credit is ATTEST → EVALUATE.
Auditing is EVALUATE over past receipts. API receipting is ATTEST.
Billing is TRANSACT. Compliance is EVALUATE.

There is no fourth act. If a new verb cannot be expressed as one of
these three, the verb is wrong.

### Four Decisions

The policy gate produces exactly one of four outcomes:

- **ALLOW** — proceed, effects will be executed
- **DENY** — rejected, reason recorded
- **REQUIRE** — needs consent before proceeding
- **GHOST** — recorded but not effectuated (Write-Before-Execute)

### Five Families

Every policy belongs to one of five families:

- **Existence** — is it well-formed?
- **Compliance** — does it meet the rules?
- **Threshold** — is the number within bounds?
- **Provenance** — was it built correctly?
- **Authorization** — can this actor do this?

---

## V. The Seven Commitments

### 1. Canonical or Nothing

There is exactly one valid byte encoding for any value.
Two implementations that disagree are both wrong until one is fixed.

### 2. Receipt or It Didn't Happen

Every state transition emits a receipt. No exceptions.
If there is no receipt, there is no proof. If there is no proof,
it did not happen.

### 3. Verify Without Trust

Any artifact produced by this system can be verified using only
the artifact itself, the signer's public key, and the NRF-1.1 spec.
No database query. No API call. No "trust me."

### 4. Ghost Before Execute

Every act with side effects begins as a Ghost(pending).
The ghost is immutable evidence of intent. If execution fails,
the ghost remains as proof that the attempt was made.
This is Write-Before-Execute. It is not optional.

### 5. Permit Before Allow

"ALLOW" without a signed Permit is a claim.
"ALLOW" with a signed Permit is math.
The Permit is time-bounded, hash-pinned, and independently verifiable.
It closes the accountability gap between "someone decided" and
"here is the cryptographic proof of who decided what, when, over what input."

### 6. Compose, Don't Orchestrate

Modules compose by emitting receipts that reference prior receipts
via `pipeline_prev`. There are no workflow engines, no message buses,
no orchestrators. Just CIDs pointing to CIDs.

The simplest composition mechanism is also the most powerful:
a pointer from one immutable artifact to another.

### 7. Products Are Data, Not Code

A product is a `product.json` manifest. It declares which acts,
which policies, which proof level, which enrichments, which billing.
If creating a new product requires writing Rust, something is wrong
with the Base or the Modules.

---

## VI. Why This Matters

### For AI

AI systems make decisions that affect people's lives, livelihoods,
and liberties. Today, those decisions are logged in proprietary formats,
stored in databases controlled by the decision-maker, and audited
(if at all) by asking the decision-maker to audit themselves.

ai-nrf1 makes AI decisions independently verifiable. The model's
judgment, the policy it was evaluated against, the input it received,
and the decision it produced — all bound into a single canonical
artifact that anyone can verify.

### For Compliance

Regulators don't want dashboards. They want proof.
ai-nrf1 produces proof that is:
- Deterministic (same input → same output → same hash)
- Signed (Ed25519 over BLAKE3 of canonical NRF bytes)
- Chained (each receipt links to the previous via CID)
- Offline-verifiable (no API call needed)
- Tamper-evident (change one bit, break the hash)

### For Commerce

Every API call, every transaction, every evaluation — receipted.
Not logged. Receipted. A receipt is not a log line. A receipt is a
signed, hashed, chained, independently verifiable artifact with a URL.

You can put it in a court filing. You can embed it in a PDF.
You can print it as a QR code. You can verify it in 2050.

### For Trust

In a world where AI agents interact with AI agents, trust cannot
be based on reputation, brand, or "we're a big company."
Trust must be based on math. ai-nrf1 provides the math.

---

## VII. The Name

**ai-nrf1** — Artificial Intelligence Normalized Representation Format, version 1.

- **ai** — because this was built for the age of AI, where machines
  make decisions and humans need to verify them
- **nrf** — Normalized Representation Format, because canonicality
  is the foundation everything else stands on
- **1** — because we got it right the first time, and the spec
  is frozen. NRF-1.1 is the final encoding. What changes is what
  you build on top of it.

The brand name is **UBL** (Universal Business Layer) for the product
surface. The spec name is **ai-nrf1** for the canonical encoding.
The Rust crate is **nrf-core** for the implementation.

Three names. One truth. The bytes are the bytes.

---

## VIII. The Hierarchy of Law

```
┌──────────────────────────────────┐
│     The Fractal Invariant        │  ← Immutable. The physics.
│  Value → NRF → BLAKE3 → CID     │
│  → Ed25519 → URL                 │
├──────────────────────────────────┤
│   Constitution of the Base       │  ← Immutable. The terrain.
│   9 Articles. 7 types. 4 arts.   │
├──────────────────────────────────┤
│   Constitution of the Modules    │  ← Stable. The roads.
│   6 Articles. 4 categories.      │
├──────────────────────────────────┤
│   Constitution of the Products   │  ← Flexible. The vehicles.
│   5 Articles. 5 dimensions.      │
├──────────────────────────────────┤
│   product.json manifests         │  ← Ephemeral. The trips.
│   Created and retired freely.    │
└──────────────────────────────────┘
```

Lower layers never change to accommodate upper layers.
Upper layers always adapt to lower layers.
This is not bureaucracy. This is physics.

---

## IX. The Canonical Equation

This is the equation that governs everything:

**One value. One message. One byte stream. One hash.**

It is not a slogan. It is a mathematical invariant:

```
∀ v: Value,
  encode(v) = encode(v)                    — determinism
  decode(encode(v)) = v                    — round-trip
  encode(decode(bytes)) = bytes            — canonicality
  BLAKE3(encode(v)) = BLAKE3(encode(v))    — hash stability
```

If two systems encode the same value and get different bytes, one of them
is broken. If two systems hash the same bytes and get different hashes,
one of them is broken. If a signature verifies on one machine but not
another, one of them is broken.

There is no "it depends." There is no "configuration." There is no
"well, in this locale..." The bytes are the bytes. The hash is the hash.
The signature is the signature.

This is what makes the system trustworthy: not the reputation of the
signer, not the brand of the provider, not the size of the company.
The math. Only the math.

---

## X. LLM-First by Design

ai-nrf1 was built for a world where AI agents make decisions, generate
artifacts, and audit each other's work. Every design choice was made
with one question: **"Can an LLM reason about this correctly?"**

### What makes NRF-1.1 LLM-friendly

- **Zero-choice encoding.** 7 tags. No variants. No floats. No optional
  semantics. An LLM doesn't need to "choose" between valid encodings
  because there is only one. This eliminates format hallucinations.

- **Fixed-width integers.** Int64 is always 1 tag byte + 8 data bytes.
  An LLM can count to 9. It cannot reliably compute variable-width
  integer encodings with 17 different size classes (looking at you, CBOR).

- **Hex-only bytes.** The JSON mapping uses `{"$bytes":"hex"}` — one
  format, lowercase, even length. No base64. No base58. No "is this
  URL-safe or standard?" An LLM can read hex. It struggles with base64.

- **Sorted maps by raw bytes.** Not locale-dependent. Not Unicode
  collation. Raw byte comparison. An LLM can sort strings by bytes.
  It cannot reliably apply locale-specific collation rules.

- **Greppable error names.** `InvalidMagic`, `NonMinimalVarint`, `NotNFC`,
  `UnsortedKeys`, `DuplicateKey`. Short, unique, unambiguous. An LLM
  can match an error to a fix without parsing a paragraph.

- **Self-describing receipts.** Every field has a clear name. `act`,
  `decision`, `body_cid`, `permit_cid`, `pipeline_prev`. An LLM can
  read a receipt and understand what happened without documentation.

- **The fractal is learnable.** The same 6 steps repeat at every scale.
  An LLM learns the pattern once and applies it everywhere. No special
  cases. No "except when..."

### What an LLM cannot do (by design)

- **Compute BLAKE3 hashes.** No LLM can hash. But an LLM CAN verify
  that a CID starts with `b3:` and contains 64 hex characters. The
  structure is checkable. The math requires a runtime.

- **Compute Ed25519 signatures.** No LLM can sign. But an LLM CAN
  verify that a signature is 64 bytes, that the `sig` field is present,
  and that the signing flow is correct (hash the NRF-without-sig, then sign).

- **Perform NFC normalization.** No LLM can reliably determine if a
  string is NFC. But the spec says "reject, don't normalize" — the LLM
  only needs to know the rule, not perform the check.

These are not limitations. These are the correct boundary between what
an LLM reasons about (structure, rules, flow) and what a runtime
computes (hashes, signatures, normalization). The LLM is the auditor.
The runtime is the calculator. Neither replaces the other.

### The LLM workflow

An LLM interacting with ai-nrf1 follows this pattern:

1. **Generate** — construct a valid `body` as JSON, following the type rules
2. **Request** — send it through the pipeline (intake → policy → runtime → receipt)
3. **Read** — parse the receipt, check field presence, verify CID format
4. **Audit** — verify that `body_cid` matches the body, that `act` is valid,
   that `pipeline_prev` references exist, that `decision` is one of four values
5. **Compose** — reference the receipt's CID in a new act via `pipeline_prev`

At no point does the LLM need to touch bytes directly. It works in JSON.
The runtime handles encoding, hashing, and signing. The LLM handles
meaning, structure, and logic.

**ai-nrf1 is not LLM-compatible. It is LLM-first.**
The difference: compatible means "an LLM can use it." First means
"we removed everything that would confuse an LLM before we shipped it."

---

## XI. The Invitation

This is not a closed system. It is a spec, an implementation,
and a set of constitutions.

If you can encode a value into NRF-1.1 bytes and produce the same
hash we do, you are compatible. If you can verify our signatures,
you can trust our receipts. If you can read our receipts, you can
audit our decisions.

We did not build a platform. We built a protocol.
We did not build a product. We built a product factory.
We did not build trust. We built the math that makes trust unnecessary.

**One value. One byte stream. One hash. One signature. One decision.**

That's the whole thing. Everything else is composition.

---

*ai-nrf1: because "trust me" is not a proof.*
