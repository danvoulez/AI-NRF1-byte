# PRODUCTS — Layer 3: Configuration

> The fractal at the organism scale.
> A product is a manifest (a Value) that composes modules into a pipeline.
> The manifest itself gets normalized, hashed, and signed. Same chain.

## Constitution

[CONSTITUTION.md](CONSTITUTION.md) — 5 articles defining:

1. A product is a manifest, not code
2. Lifecycle: init → configure → deploy → update
3. Categories: compliance, identity, commerce, AI
4. Rules: products cannot modify BASE or MODULES
5. Factory: products are born from templates via the product spitter

## How It Works

```
product.json (manifest)
  → pipeline: [cap-intake, cap-policy, cap-enrich, cap-transport]
  → each step: Value → ρ(Value) → receipt
  → chain receipts → capsule → CID
```

Products live in **separate repos**. This repo provides:

1. **Registry service** — HTTP API that runs pipelines (`services/registry/`)
2. **Product factory** — generates new product repos (`tools/product-spitter/`)
3. **UI template** — skeleton for product frontends (`services/ui-template/`)

## Design Documents

| Doc | Topic |
|-----|-------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Product layer architecture |
| [FACTORY.md](FACTORY.md) | Product spitter / factory system |
| [GENERATOR-GUIDE.md](GENERATOR-GUIDE.md) | Guide for generating products |
