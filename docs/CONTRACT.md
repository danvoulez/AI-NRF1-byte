# API Contract v0

All product UIs (mother and children) communicate with the registry via these endpoints.
The `v0` prefix signals this contract is pre-stable and may change with notice.

Base URL: `NEXT_PUBLIC_REGISTRY_URL` (default `http://localhost:4000`)

---

## Endpoints

### `GET /api/v0/whoami`

Returns registry identity and detected caller context.

**Request headers** (optional):
- `X-Tenant` — tenant slug
- `X-Product` — product slug

**Response** `200 OK`:
```json
{
  "api_version": "v0",
  "registry_version": "1.0.0",
  "git_sha": "abc1234",
  "tenant": "acme",
  "product": "acme-verify",
  "modules": true
}
```

---

### `POST /api/v0/run`

Execute a product pipeline (manifest + env + tenant).

**Request body**:
```json
{
  "manifest": {
    "name": "cap-intake-demo",
    "version": "1.0.0",
    "pipeline": [
      { "step_id": "intake", "kind": "cap-intake", "config": {} }
    ]
  },
  "env": { "document_url": "https://example.com/doc.pdf" },
  "tenant": "default"
}
```

**Response** `200 OK`:
```json
{
  "ok": true,
  "verdict": "Allow",
  "stopped_at": null,
  "receipt_cid": "b3:abc123...",
  "receipt_chain": ["b3:abc123..."],
  "url_rica": "https://resolver.local/r/b3:abc123...",
  "hops": [
    { "step": "intake", "kind": "cap-intake", "hash": "b3:abc123...", "verified": true }
  ],
  "metrics": [
    { "step": "intake", "metric": "duration_ms", "value": 42 }
  ],
  "artifacts": 1
}
```

**Error** `400 Bad Request`:
```json
{ "ok": false, "error": "invalid manifest: ..." }
```

---

### `GET /api/v0/executions`

List stored executions (most recent first, max 500).

**Response** `200 OK`:
```json
[
  {
    "id": "exec_1707750000000",
    "state": "ACK",
    "cid": "b3:abc123...",
    "title": "cap-intake-demo",
    "origin": "api-gateway",
    "timestamp": "2026-02-12T13:00:00Z",
    "integration": "SDK"
  }
]
```

---

### `GET /api/v0/receipts/:cid`

Receipt detail with SIRP timeline, proofs, and evidence.

The `:cid` path parameter should be URL-encoded by the client.
The server accepts both raw (`b3:abc123`) and URL-encoded (`b3%3Aabc123`) forms.

**Response** `200 OK`:
```json
{
  "execution": {
    "id": "exec_1707750000000",
    "state": "ACK",
    "cid": "b3:abc123...",
    "title": "cap-intake-demo",
    "origin": "api-gateway",
    "timestamp": "2026-02-12T13:00:00Z",
    "integration": "SDK"
  },
  "sirp": [
    {
      "step": "INTENT",
      "signer": "engine:cap-intake@1.0.0",
      "timestamp": "2026-02-12T13:00:00Z",
      "verified": true,
      "algorithm": "Ed25519",
      "hash": "b3:abc123..."
    }
  ],
  "proofs": [
    {
      "type": "Capsule INTENT",
      "algorithm": "Ed25519",
      "cid": "b3:abc123...",
      "signer": "engine:cap-intake@1.0.0",
      "timestamp": "2026-02-12T13:00:00Z"
    }
  ],
  "evidence": [
    {
      "cid": "b3:abc123...",
      "url": "https://resolver.local/e/b3:abc123...",
      "status": "fetched",
      "mime": "application/json"
    }
  ]
}
```

**Error** `404 Not Found`:
```json
{ "error": "receipt b3:abc123... not found" }
```

---

### `GET /api/v0/metrics`

Dashboard statistics.

**Response** `200 OK`:
```json
{
  "executionsToday": 42,
  "ackPercentage": 95.2,
  "p99Latency": 120,
  "activeIntegrations": 1,
  "weeklyData": []
}
```

---

## Non-versioned endpoints

These are infrastructure endpoints, not part of the product API contract:

| Endpoint | Description |
|---|---|
| `GET /health` | `{"status":"ok"}` — load balancer health check |
| `GET /healthz` | Alias for `/health` |
| `GET /readyz` | Alias for `/health` |
| `GET /version` | `{"version","git_sha","build_ts","modules"}` |

---

## CID encoding

- CIDs use the `b3:` prefix followed by hex-encoded blake3 hash
- Clients MUST `encodeURIComponent(cid)` when passing CIDs in URL paths
- Server URL-decodes the path parameter (accepts both raw and encoded)
- Example: `b3:abc123` → path `/api/v0/receipts/b3%3Aabc123`

---

## Future (v1)

The following will be added before graduating to `v1`:
- `X-Tenant` + `X-Product` headers required on all `/api/*` calls
- API key authentication per product
- `GET /api/v0/audits`, `GET /api/v0/evidence`, `GET /api/v0/policies`
