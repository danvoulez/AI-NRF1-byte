# Product Generator Guide

> The fractal at the organism scale.
> A product is a `product.json` manifest. It declares which modules, which policies, which proof level.
> If creating a product requires writing Rust, something is wrong with BASE or MODULES.

## What the Generator Does

`ubl product from-manifest` transforms a **product manifest** (`product.json`) + an optional **lock file** (`modules.lock.json`) into a **self-contained binary** (or container) that:

- Exposes `POST /evaluate` (receives JSON, runs pipeline, emits receipt with CID)
- Signs and chains receipts, returns `url_rica`
- Runs **without product-specific code** (no-code, manifests only)

## Product Manifest (`product.json`)

```json
{
  "v": 1,
  "name": "tdln-policy",
  "version": "1.0",
  "pipeline": [
    {
      "step_id": "step-0-intake",
      "kind": "cap-intake",
      "version": "0",
      "config": { "map": [{ "from": "data", "to": "payload" }] }
    },
    {
      "step_id": "step-1-policy",
      "kind": "cap-policy",
      "version": "0",
      "config": { "rules": [{ "EXIST": ["payload"] }] }
    },
    {
      "step_id": "step-2-enrich",
      "kind": "cap-enrich",
      "version": "0",
      "config": { "providers": ["status_page"] }
    },
    {
      "step_id": "step-3-transport",
      "kind": "cap-transport",
      "version": "0",
      "config": { "node": "did:ubl:local", "relay": [] }
    }
  ],
  "outputs": { "format": "json" }
}
```

### Required fields

- **v** — schema version (currently `1`)
- **name** — product name (lowercase, hyphens)
- **version** — product version string
- **pipeline** — ordered list of steps, each with:
  - **step_id** — unique identifier
  - **kind** — capability name (`cap-intake`, `cap-policy`, `cap-enrich`, `cap-transport`, `cap-permit`, `cap-pricing`, `cap-runtime`, `cap-llm`)
  - **version** — major version (`"0"`) or wildcard (`"*"`)
  - **config** — capability-specific configuration

### Aliases

You can use aliases in authoring; the generator resolves them:

| Alias | Resolves to |
|-------|-------------|
| `policy.light` | `cap-policy` |
| `cap-structure` | `cap-intake` |
| `cap-llm-engine` | `cap-llm` (with `provider_hint: "engine"`) |
| `cap-llm-smart` | `cap-llm` (with `provider_hint: "smart"`) |

## Module Lock (`modules.lock.json`)

Pins each capability by BLAKE3 CID of its bytes. Guarantees reproducibility.

```json
{
  "spec": 1,
  "locked_at": "2026-02-12T11:22:33Z",
  "modules": {
    "cap-intake@0": { "cid": "b3:...", "bytes_len": 12345 },
    "cap-policy@0": { "cid": "b3:...", "bytes_len": 23456 }
  }
}
```

If a lock is present, the generator verifies that `kind@major` CIDs match. Mismatch → hard failure (determinism above all).

## CLI Commands

### Run a product locally

```bash
ubl product from-manifest \
  --manifest ./products/tdln-policy/product.json \
  --vars tenant=acme priority=normal \
  --port 8787
```

### Generate a lock file

```bash
ubl product lock \
  --manifest ./products/tdln-policy/product.json \
  --out ./products/tdln-policy/modules.lock.json
```

### Explain the pipeline (dry run)

```bash
ubl product explain --manifest ./products/tdln-policy/product.json
```

### Build a container

```bash
ubl product from-manifest \
  --manifest ./products/tdln-policy/product.json \
  --container
```

## HTTP API (generated binary)

### `POST /evaluate`

Request: JSON (becomes `env` via json_view; `--vars` also enter `env`)

```json
{ "data": "hello", "tenant": "acme" }
```

Response (200):

```json
{
  "product": "tdln-policy",
  "verdict": "Allow",
  "receipt_cid": "b3:...",
  "url_rica": "https://resolver.local/r/b3:...",
  "artifacts": [{ "kind": "status_page", "cid": "b3:..." }],
  "metrics": [{ "step": "step-1-policy", "metric": "duration_ms", "value": 0 }]
}
```

### `GET /health` → `200 OK`

### `GET /version` → `{ "version": "...", "git_sha": "...", "build_ts": "..." }`

### Errors

All errors follow the canonical shape:

```json
{
  "ok": false,
  "error": {
    "code": "Err.Config.InvalidManifest",
    "message": "step_id 'step-1' duplicated",
    "hint": "Ensure all step_id values are unique within the pipeline",
    "status": 400
  }
}
```

## Execution Modes

| Mode | Description | Use case |
|------|-------------|----------|
| **Stub** | Fast compile, no IO | Local dev, CI |
| **Runner-real** | Real pipeline with 8 modules | Smoke tests, E2E |
| **Container** | OCI image with generated binary | Production deploy |

## Architecture Note

Products are **NOT** part of this repo. They are independent repos/binaries that consume BASE + MODULES via HTTP. This repo provides:

1. **Registry service** — HTTP API for running pipelines
2. **Product factory** (`tools/product-spitter/`) — generates new product repos
3. **CLI** (`ubl product from-manifest`) — dev/testing tool

The `ubl-cli` with `runner-real` is a development tool only. In production, products call the registry service over the wire.

## 5-Minute Quickstart

```bash
# 1. Build with real runner
cargo build -p ubl-cli --features runner-real

# 2. Run a product
ubl tdln policy --var data=hello

# 3. Or run via manifest
ubl product from-manifest --manifest products/tdln-policy/product.json --port 8787

# 4. Test it
curl -s -XPOST http://localhost:8787/evaluate \
  -H "Content-Type: application/json" \
  -d '{"data":"hello"}' | jq

# 5. Generate lock for production
ubl product lock --manifest products/tdln-policy/product.json --out modules.lock.json
```
