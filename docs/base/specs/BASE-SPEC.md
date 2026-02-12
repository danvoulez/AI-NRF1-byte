#1 — BASE (ai-nrf1 + ai-json-nrf1 + ubl-capsule v1)

> **Missão:** ser o “chão de fábrica” determinístico de bytes e provas, onde tudo acima (módulos e produtos) confia sem pestanejar.

---

## 0) Visão Rápida

* **ai-nrf1**: formato binário canônico (canon = verdade).
* **ai-json-nrf1**: *view* humana/LLM derivada do canon, 1:1 sem ambiguidade.
* **ubl-capsule v1**: mensagem com **id estável**, **seal assinado**, **cadeia de receipts** (hops) — tudo em ai-nrf1.

**Invariantes centrais**

* Canonicidade forte (mesmo valor ⇒ mesmos bytes).
* ID = BLAKE3 sobre **bytes canônicos sem `id` e sem assinaturas**.
* Seal assina **domínio + id + hdr + env** (separação de domínio).
* Receipts encadeadas (`prev`), append-only, cada uma assinada.
* **WBE + HAL**: recibo antes do efeito; limites duros de CPU/mem/IO/tempo.

---

## 1) Contratos (normativos)

### 1.1 ai-nrf1 (wire)

Tipos (tags fixos):

* `00 null`, `01 false`, `02 true`, `03 int64(8B)`, `04 string(varint32 + UTF-8/NFC, sem BOM)`,
  `05 bytes(varint32 + octets)`, `06 array(varint32 + N valores)`, `07 map(varint32 + N pares key:String,value)`.

Regras:

* **Strings**: UTF-8 válida, **NFC**, sem BOM.
* **Numéricos**: **Int64** (dois complementos, 8 bytes big-endian). Sem float.
* **varint32**: **LEB128 mínimo** (1–5 bytes).
* **Map**: chaves String, **ordenadas por bytes UTF-8**, únicas.
* **Set (em nível ρ)**: ordenar por bytes ai-nrf1 do item, dedup vizinho.

### 1.2 ai-json-nrf1 (view)

* **Bytes** como `b3:<hex>` (32B comuns) ou `b64:<...>` quando for payload arbitrário.
* Aceita escapes JSON (`\u0061`) mas materializa como bytes NFC + UTF-8.
* Parser **MUST** validar: **NFC**, ASCII-only onde indicado (DIDs/KIDs), sem float.

### 1.3 ubl-capsule v1 (objeto canônico)

```jsonc
{
  "v": "ubl-capsule/1.0",
  "id": Bytes(32),                 // blake3(capsule \ {id, seal.sig, receipts[*].sig})
  "hdr": {
    "src": "DID#KEY",              // ASCII-only
    "dst": "DID#KEY",              // ASCII-only
    "nonce": Bytes(16),
    "exp": Int64,                  // epoch-nanos
    "chan": "ASCII-chan"?, "ts": Int64?
  },
  "env": {                         // ai-json-nrf1 v0.1.1
    "v": "ai-json-nrf1/0.1.1",
    "t": "record"|"bundle"|"trace",
    "agent": {"id": String, "name": String?},
    "intent": {"kind":"ATTEST"|"EVAL"|"BUNDLE"|"TRACE"|"QUERY","name":String,"args":Map?},
    "ctx": Map,
    "decision": {"verdict":"ACK"|"NACK"|"ASK","reason":String?,"metrics":Map?},
    "evidence": {"cids":[Bytes(32)]?,"urls":[String]?},
    "meta": {"app":String,"tenant":String,"user":String,"session":String?}?,
    "links": {"prev":Bytes(32)?,"trace":Bytes(32)?}?
  },
  "seal": {
    "alg": "Ed25519"|"Dilithium3",
    "kid": "DID#KEY",              // ASCII-only
    "domain": "ubl-capsule/1.0",
    "scope": "capsule",
    "aud": "DID#KEY"?,             // opcional, bind ao dst
    "sig": Bytes(64|..)
  },
  "receipts": [                    // hop receipts (append-only)
    {
      "of": Bytes(32),             // == capsule.id
      "prev": Bytes(32),           // hash do hop anterior (ou zero no primeiro)
      "kind": "relay"|"exec"|"dlv"|"ack"|...,
      "node": "DID#KEY",
      "ts": Int64,                 // epoch-nanos
      "sig": Bytes(64|..)
    }
  ]?
}
```

**Assinaturas**

* **Seal** assina `blake3(nrf.encode({domain,id,hdr,env}))`.
* **Receipt** assina `blake3(nrf.encode({domain:"ubl-receipt/1.0", of, prev, kind, node, ts}))`.

---

## 2) APIs da BASE (Rust + CLI + WASM)

### 2.1 Crates (layout sugerido)

```
impl/rust/
  ai-nrf1/          # encode, decode, hash, errors
  ai-json-nrf1/     # to_json, from_json (view)
  ubl-capsule/      # id(), sign(), verify(), add_receipt(), verify_chain()
  ubl-verify/       # bundle verify offline, receipts chain, policy invariants
```

### 2.2 Assinaturas centrais (Rust)

```rust
// ai-nrf1
fn encode(v: &Value) -> Result<Vec<u8>>;
fn decode(bytes: &[u8]) -> Result<Value>;
fn hash(bytes: &[u8]) -> [u8;32]; // BLAKE3

// ai-json-nrf1
fn to_json(v: &Value) -> serde_json::Value;
fn from_json(j: &serde_json::Value) -> Result<Value>; // valida NFC/ASCII/no-float

// ubl-capsule
fn capsule_id(mut capsule: Value) -> Result<[u8;32]>;                 // exclui id/sigs
fn sign(capsule: &mut Value, kp: &KeyPair) -> Result<()>;             // preenche id e seal.sig
fn verify(capsule: &Value, pk: &PublicKey) -> Result<()>;             // confere id + seal
fn add_receipt(capsule: &mut Value, hop: &Hop, kp: &KeyPair) -> Result<()>;
fn verify_chain(capsule: &Value, chain_policy: &ChainPolicy) -> Result<()>;
```

### 2.3 ABI WASM (para módulos e verificadores externos)

* **Puro, buffer-in/buffer-out**:

  * `fn eval(input_ai_nrf1: &[u8]) -> Result<Vec<u8>>`
  * `fn verify_capsule(input_ai_nrf1: &[u8]) -> Result<Vec<u8>>` (retorna relatório ai-json-nrf1)

### 2.4 CLI (mínimo viável)

```
ubl cap canon      <in.json>  -> stdout .nrf        # json -> ai-nrf1
ubl cap view-json  <in.nrf>   -> stdout .json       # ai-nrf1 -> view
ubl cap hash       <in.nrf>   -> b3:<hex>
ubl cap sign       --key <did#k> <in.json> -> out.nrf
ubl cap verify     <in.nrf>   -> OK / erro
ubl cap receipt add --kind relay --node <did#k> --key <did#k> <in.nrf> -> out.nrf
ubl cap verify-chain <in.nrf> -> OK / relatório
```

---

## 3) Segurança & Confiabilidade

* **Sem floats** no canon.
* **Bytes p/ cripto** (id, sigs, nonce, cids).
* **ASCII-only** p/ DIDs/KIDs.
* **Anti-replay**: cache `(hdr.src, hdr.nonce)` por janela, e checagem de `hdr.exp`.
* **Sandbox** para qualquer código de verificação externo (WASM).
* **WBE + HAL**: sem efeito sem recibo; limites duros no executor.
* **Domain-separation** explícito nas assinaturas (strings de domínio).

---

## 4) Desempenho & SLO

* Encode/Decode O(n). Alvo: **P50 < 200µs**, **P99 < 2ms** p/ objetos ≤ 64KB.
* Verificação de cadeia (≤ 64 hops): **P99 < 5ms**.
* Hash BLAKE3 linear (vetorado quando possível).

---

## 5) Observabilidade

* **Logs estruturados**: `capsule_id`, `card_cid`, `tenant`, `ulid`, `act`, `hop_kind`.
* **Métricas**: `canon_fail_total`, `seal_verify_ms`, `receipts_len`, `verify_chain_ms`.
* **Tracing**: correlação por `capsule_id` e `ulid` (se presente).

---

## 6) Testes (KATs, Propriedades, Integração)

### 6.1 KATs (vetores canônicos)

* Unicode: NFC vs NFD, rejeição de BOM, `NotNFC`.
* varint minimal (1..5B), `NonMinimalVarint`.
* Map ordering (UTF-8 bytes), duplicatas (`DuplicateKey`).
* **Empty bytes** (`{"$bytes":""}`) idempotente.
* Timestamps RFC-3339Z (view) → Int64 nanos (canon).
* Receipts: cadeia `prev` correta; alteração detectada.

### 6.2 Propriedades

* **Idempotência**: `ρ(ρ(x))=ρ(x)`; `encode(json→canon→json)` estável.
* **Confluência**: ordem de normalização irrelevante.
* **Canonicidade**: `v1 ≡ v2` ⇒ `encode(ρ(v1)) == encode(ρ(v2))`.
* **ID estável**: add/remove receipt **não** altera `id`.

### 6.3 Integração

* `json → nrf → verify → add_receipt → verify_chain → view-json`.
* `ASK` ⇒ `links.prev` presente; `ACK/NACK` ⇒ `evidence` presente (pode vazio).
* **Offline bundle** (se aplicável ao verificador).

---

## 7) CI (gates obrigatórios)

### 7.1 Lint & Build

* `cargo fmt --all`
* `cargo clippy --workspace --all-targets -- -D warnings`
* `mypy/ruff` pros bindings Py (se houver)

### 7.2 Tests

* Unit + Prop + KATs + Integração.
* Fuzz (decoder ai-nrf1) com seed Unicode/varint ≥ 30min.
* **Falhas de canonicidade/assinatura = bloqueio de merge**.

### 7.3 Exemplo (GitHub Actions, excerto)

```yaml
name: base-ci
on: [push, pull_request]
jobs:
  build-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace --all-features -- --nocapture
  fuzz-nrf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cargo-fuzz
      - run: cargo fuzz run nrf_decode -- -max_total_time=1800
```

---

## 8) Versionamento & Compatibilidade

* **Wire major** fixo: `ai-nrf1/1` e `ubl-capsule/1.0`.
* Novas regras **só** se não quebram canonicidade; caso contrário, `v2`.
* Schemas e docs com **CIDs** (BLAKE3) para *immutability receipts*.

---

## 9) Tabela de Erros (canonical)

* `Err.Canon.InvalidUTF8`, `Err.Canon.NotNFC`, `Err.Canon.BOMPresent`
* `Err.Canon.FloatForbidden`, `Err.Canon.NonMinimalVarint`
* `Err.Canon.NonStringKey`, `Err.Canon.UnsortedKeys`, `Err.Canon.DuplicateKey`
* `Err.Capsule.IDMismatch`, `Err.Seal.BadSignature`, `Err.Seal.ScopeDomain`
* `Err.Hop.BadChain`, `Err.Hop.BadSignature`, `Err.Hdr.Expired`

---

## 10) Exemplos (curtos)

**String “hello”**

```
6E726631 04 05 68656C6C6F
```

**Map {"a":1,"b":true} (ordenado)**

```
6E726631 07 02
  04 01 61  03 0000000000000001
  04 01 62  02
```

**Soma nullária {"$case":"Foo"} (view) → ai-nrf1 (esqueleto)**

```
6E726631 07 01
  04 05 2463617365   // "$case"
  04 03 466F6F       // "Foo"
```

---

## 11) Definition of Done (BASE)

* [ ] KATs: Unicode/varint/map/empty-bytes/timestamps/receipts
* [ ] Id/Seal/Receipts validados (tamper detection)
* [ ] Fuzz decoder ai-nrf1 (≥30min) sem crash
* [ ] CLI mínima operante (canon/hash/sign/verify/receipt)
* [ ] Docs + Schemas publicados com CIDs
* [ ] Observabilidade: métricas e logs estruturados básicos

---

