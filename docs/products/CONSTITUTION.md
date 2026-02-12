# Constitution of the Products

*The laws governing how modules are assembled into shipped, billed, marketed things.*
*A product that violates these laws is not a product — it is a liability.*

---

## Preamble

A Product is a configuration, not a codebase.

The Base provides the terrain. Modules build the roads.
A Product is a map that says: "Start here, take this road, then that road,
arrive there, and charge this much."

The moment a product requires custom infrastructure that doesn't exist
in the Base, or custom logic that doesn't exist in a Module, the product
is telling you that the Base or a Module is incomplete — not that the
product needs special treatment.

---

## Article I — A Product Is a Manifest

### Section 1.1 — The product.json

Every product is defined by a single `product.json` file:

```json
{
  "name": "ai-model-passport",
  "version": "1.0.0",
  "acts": [
    { "act": "ATTEST", "intake": "intake-model-card", "policy": "policy-existence" },
    { "act": "EVALUATE", "intake": null, "policy": "policy-eu-ai-act" }
  ],
  "pipeline": ["ATTEST", "EVALUATE"],
  "proof_level": "SIRP",
  "enrichments": ["card-url", "badge", "pdf"],
  "billing": "per_evaluation",
  "consent": null
}
```

### Section 1.2 — No Product-Specific Code

If a product requires a line of Rust that is not in a Base crate or a Module,
that line belongs in a Module, not in the product. Products are JSON.
Products are configuration. Products are data.

The only code a product may contain is:
- Its `product.json` manifest
- Integration tests that prove the manifest works
- Documentation

### Section 1.3 — Product = Acts + Policies + Proof + Enrichments + Billing

Every product is fully described by five dimensions:

| Dimension | Options |
|-----------|---------|
| **Acts** | ATTEST, EVALUATE, TRANSACT (and their pipeline order) |
| **Policies** | Which policy modules to invoke, in what order |
| **Proof Level** | Receipt, SIRP, or Bundle |
| **Enrichments** | Card URL, Badge, Status Page, PDF, Ghost, Webhook |
| **Billing** | Per evaluation, per receipted event, per policy pack, per artifact |

If you cannot describe your product using these five dimensions,
your product is either too complex (split it) or your modules are
incomplete (build the missing module first).

---

## Article II — Product Lifecycle

### Section 2.1 — The Universal Pipeline

Every product, regardless of what it does, follows the same pipeline:

```
INPUT → CANON → POLICY GATE → CERTIFIED RUNTIME → DECISION → RECEIPT → URL
```

The product manifest configures WHICH modules fill each stage.
The pipeline itself is immutable.

### Section 2.2 — State Machine

Every act within a product follows this state machine:

```
NEW → GHOST(pending)
GHOST(pending) → POLICY GATE
POLICY GATE returns ALLOW → READY → RUNNING → DONE (Receipt emitted)
POLICY GATE returns DENY → DONE (Receipt emitted with decision=DENY)
POLICY GATE returns REQUIRE → CONSENT QUEUE → (k-of-n grants) → READY
POLICY GATE returns GHOST → GHOST(pending) remains (WBE only)
ERROR at any stage → DONE (Receipt emitted with error in effects)
```

### Section 2.3 — Multi-Act Products

Products with multiple acts in their pipeline execute them in order.
Each act produces a Receipt. The next act references the previous
Receipt's CID via `pipeline_prev`.

```
Act 1 (ATTEST)   → Receipt_A
Act 2 (EVALUATE)  → Receipt_B (pipeline_prev: [Receipt_A.receipt_cid])
```

The product is complete when all acts in the pipeline have produced receipts.

---

## Article III — Product Categories

These are not exhaustive. They are the first wave — proof that the
architecture works. Every future product follows the same pattern.

### Section 3.1 — API Receipt Gateway

The simplest product. Universal pain. Proves the stack.

```json
{
  "name": "api-receipt-gateway",
  "acts": [{ "act": "ATTEST", "policy": "policy-existence" }],
  "pipeline": ["ATTEST"],
  "proof_level": "Receipt",
  "enrichments": ["card-url"],
  "billing": "per_receipted_event"
}
```

**What it does:** Any API call gets a signed, verifiable receipt.
**Who buys it:** Every API provider who needs audit trails.

### Section 3.2 — AI Model Passport

Highest value per deal. Needs compliance pack.

```json
{
  "name": "ai-model-passport",
  "acts": [
    { "act": "ATTEST", "policy": "policy-existence" },
    { "act": "EVALUATE", "policy": "policy-eu-ai-act" }
  ],
  "pipeline": ["ATTEST", "EVALUATE"],
  "proof_level": "SIRP",
  "enrichments": ["card-url", "badge", "pdf"],
  "billing": "per_evaluation"
}
```

**What it does:** Attests model properties, evaluates against EU AI Act.
**Who buys it:** AI companies shipping to Europe.

### Section 3.3 — Insurance Underwriting

Composed product. Two acts, one pipeline.

```json
{
  "name": "insurance-underwriting",
  "acts": [
    { "act": "ATTEST", "policy": "policy-existence" },
    { "act": "EVALUATE", "policy": "policy-risk-threshold" }
  ],
  "pipeline": ["ATTEST", "EVALUATE"],
  "proof_level": "Bundle",
  "enrichments": ["pdf", "status-page"],
  "billing": "per_evaluation"
}
```

**What it does:** Attests claim data, evaluates against risk thresholds.
**Who buys it:** Insurers who need auditable underwriting decisions.

---

## Article IV — Product Rules

### Section 4.1 — Every Product Emits Receipts

A product that does not emit at least one Receipt per invocation
is not a product. It is a function call. We don't sell function calls.

### Section 4.2 — Every Product Has a URL

Every receipt produced by a product MUST have a rich URL:

```
https://passports.ubl.agency/{app}/{tenant}/receipts/{id}.json#cid=...&did=...&act=...
```

This URL is the product's output. It is what the customer receives.
It is what auditors verify. It is what courts accept.

### Section 4.3 — Every Product Has a Billing Axis

A product without a billing axis is a charity. We are not a charity.
Every product MUST declare how it charges:

- **Per evaluation** — each policy gate invocation
- **Per receipted event** — each receipt stored
- **Per policy pack** — monthly access to a policy module
- **Per artifact** — each enrichment produced (PDF, badge, etc.)

### Section 4.4 — Products Do Not Fork

Two products that need the same module use the SAME module.
If Product A needs a slightly different version of Module X,
the answer is to make Module X configurable, not to fork it.

### Section 4.5 — Products Are Testable Without Infrastructure

Every product MUST be testable with:
- An in-memory database (or SQLite)
- A local signing key
- No network access

If a product requires a live Postgres, a live S3, or a live API
to run its tests, the product's test suite is unconstitutional.

---

## Article V — The Product Factory

### Section 5.1 — New Products Are Cheap

Creating a new product MUST require:
1. Writing a `product.json` (minutes)
2. Ensuring the required modules exist (hours to days, one-time)
3. Writing integration tests (hours)
4. Deploying (the registry service handles it)

If creating a new product requires writing new Rust code,
something is wrong with the Base or the Modules, not with the product.

### Section 5.2 — The 16-Product Test

The architecture MUST support at least 16 distinct products using
only the Base crates and a reasonable number of modules. If it cannot,
the Base is too rigid or the module boundaries are wrong.

The first 16:
1. API Receipt Gateway
2. AI Model Passport
3. Supply Chain Notarization
4. Chip/Device Registry
5. Insurance Underwriting
6. Credit Decisioning
7. Compliance Auditing
8. SLA Monitoring
9. FINRA Trade Receipting
10. HIPAA Access Logging
11. Proof-of-Access Gateway
12. Metered Billing Receipts
13. Social Trust Score
14. Document Notarization
15. Webhook Certification
16. Multi-Party Consent Ledger

All 16 are compositions of ATTEST, EVALUATE, and TRANSACT.
All 16 use the same Receipt struct. All 16 follow the same pipeline.

---

## Ratification

```
This constitution governs all code in the PRODUCT phase.
It builds upon the Constitution of the Base and the Constitution of the Modules.
It never contradicts either.
A product is configuration. A product is a manifest. A product is data.
The code lives in the Base and the Modules. The product just points at it.
```
