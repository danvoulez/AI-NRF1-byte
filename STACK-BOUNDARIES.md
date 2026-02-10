# STACK-BOUNDARIES — Rust × TypeScript × JSON/YAML × WASM

## 0) Princípios não negociáveis

* **A verdade são os bytes**: canônico = `ai-nrf1` (NRF). Hash/ID/assinaturas **sempre** sobre bytes canônicos.
* **1 fonte de canonicidade**: encoder/decoder, BLAKE3, Ed25519/Dilithium vivem **em Rust**.
  Qualquer outro ambiente usa **WASM** gerado dessa mesma base.
* **View ≠ Canon**: `ubl-json` (view humana/LLM) é **derivada**. JSON/TS jamais “inventam” canon.
* **Políticas e decisão no hot-path**: execução determinística (Rust/WASM sandbox). UI/SDKs orquestram.

---

## 1) BASE (bytes, canon, cripto)

### 1.1 Rust (fonte da verdade)

* **Crates**:

  * `ai-nrf1-core`: tipos, encoder/decoder, validações (UTF-8, NFC, varint minimal), `Int64`, `Bytes`.
  * `ai-json-nrf1-view`: projeção JSON↔canon (apenas view segura), marshaling controlado.
  * `ubl-capsule`: `capsule_id`, `seal.sign/verify`, receipts encadeadas, verificação de cadeia.
  * `ubl-crypto`: BLAKE3, Ed25519, Dilithium3, K-of-N (se aplicável).
* **Binários**:

  * `ubl`: `cap canon|hash|sign|verify|view-json|from-json|to-json`.
  * `nrf1`: `encode|decode|hash|verify` (dev tools).
* **Teste/Conformidade**:

  * KATs, fuzz, diferencial Python↔Rust (somente para **verificar**, não para gerar canon).

### 1.2 WASM (bindings do core)

* **Build**: `wasm-pack build` (ou `cargo wasi` para runtimes restritos).
* **Pacotes gerados**:

  * `@ubl/ai-nrf1-wasm`: `encode(bytes|jsonView)`, `decode`, `hash`, `sign`, `verify`, `capsule.verifyChain`.
* **Regra**: qualquer verificação ou serialização de canon no TS/Browser/Node **passa pelo WASM**.

### 1.3 JSON/YAML (esquemas e manifests)

* **JSON Schema**:

  * `schemas/ai-json-nrf1.vX.json` (view semântica).
  * `schemas/ubl-capsule.v1.json` (cápsula).
* **YAML/JSON**:

  * `product.manifest.json|yaml` (declara produto: acts, packs, proof level, enrichments, consent k-of-n).
  * `module.manifest.json|yaml` (declara módulo e sua ABI).
* **Proibição**: **não** existe “serializer YAML/TS → canon”. Sempre: YAML/JSON → **Rust/WASM** → canon.

---

## 2) MÓDULOS (código “normal”, semântica, policy packs)

### 2.1 Rust (execução determinística)

* **Domain engines / wrappers**:

  * `wrapper-template`: `schema.rs`, `mapper.rs`, `enrichment.rs`, `hooks.rs`.
  * **Policies**: arquivos JSON/TDLN/WASM-guest, carregados por `engine-core`.
* **Executores**:

  * `engine-exec-wasm`: sandbox (WASI), sem I/O fora do gate; deterministic flags.
* **Interface**: módulos expõem **ABI estável** (capabilities) para o orquestrador (TS pode apenas orquestrar).

### 2.2 TypeScript (orquestração & edge)

* **Onde cabe TS**:

  * **SDKs** (`@ubl/sdk`, `@ubl/verify`, `@ubl/types`).
  * **Middlewares** (Express/Next/Hono) — receipt do API Gateway, roteamento multi-tenant, backpressure.
  * **UI/Enrichments** (status page, badge, ghost).
  * **Edge gateways** (Cloudflare/Vercel): terminam HTTP, validam inputs superficiais, **enfileiram** (SQS/Kafka) para workers Rust.
* **Onde TS não entra**:

  * Hot-path de canonicidade/assinatura/decisão. TS **nunca** recalcula canon/ID sem WASM.

### 2.3 JSON/YAML (config/policy)

* **Policy-packs**: JSON/TDLN/DSL, lidos pelos wrappers (Rust).
  TS pode **gerar/editar** manifests, **nunca** executar política final.

### 2.4 WASM (plugins)

* **Policies guest**: unidades em WASM (sandbox); linguagem-fonte opcional (Rust/Go/AssemblyScript), compiladas para WASM.
* **Verificação de plugin**: assinatura de módulo WASM (selo), registro de versionamento e capabilities.

---

## 3) PRODUTOS (binários dedicados)

### 3.1 Rust (serviço de produto)

* Cada produto = **um binário** (repo próprio) que:

  * Carrega `product.manifest.*`.
  * Amarra MÓDULOS necessários (dinâmicos via registry ou estaticamente via features).
  * Expõe API HTTP `/v1/run` e webhooks.
  * Usa BASE para cápsula/ID/assinaturas.
* **Comunicação**:

  * **BASE ↔ MÓDULOS**: chamada direta (processo) + ABI/traits; canon/receipts sempre pela BASE.
  * **MÓDULOS ↔ PRODUTO**: link estático (features) ou **registry** (dinâmico), versão pinada por CID.

### 3.2 TypeScript (DX & UI do produto)

* **Painel Admin/Status Page** (Next.js).
* **SDK cliente** para on-boarding do cliente (envio de intents, leitura de decisions, bundling).
* **Webhooks** consumidores (do lado do cliente).

### 3.3 JSON/YAML (manifest/ops)

* `product.manifest.yaml`: acts (ATTEST/EVALUATE/TRANSACT), packs, proof level, enrichments, consent k-of-n, billing axis, routes.
* `routes.yaml`: mapeia endpoints públicos → acts/pipelines internos.

### 3.4 WASM (verificação offline)

* `@ubl/verify` (TS) usa WASM para validar bundle/cápsula **sem chamar servidor** (Court/offline).

---

## 4) Comunicação & Orquestração

### 4.1 BASE ↔ MÓDULOS (in-process)

* **Forma**: traits/ABI em Rust; passagem de **canon** `ai-nrf1` ou `Value` normalizado.
* **Contratos**:

  * `DomainRequest`/`DomainResponse` (Rust).
  * `capsule(core)` como primeira/última milha (canon + id + seal).

### 4.2 MÓDULOS ↔ PRODUTO

* **Estático (features)**: um binário com módulos “ligados” por feature flags (P0).
* **Dinâmico (registry)**: resolução por `module.manifest` (CID) + assinatura (P1).
* **Mensageria**: quando async: fila (SQS/Kafka), chave idempotente = `capsule.id`.

### 4.3 Gateways (TS/edge) ↔ Produto (Rust)

* **HTTP/GRPC** thin; validação superficial (schema view), `capsule_id`/`verify` via WASM; push para fila.
  O **produto** consome, decide (Rust), e publica `card_url`, webhooks, bundle.

---

## 5) CLI & Codegen

### 5.1 CLI (Rust)

* `ubl product new --manifest …` → gera serviço Rust com MÓDULOS escolhidos.
* `ubl cap …` → canonicidade, hash, assinatura, receipts.
* **Sem** geração de canon no TS.

### 5.2 Codegen (TS)

* `jsonschema → @ubl/types` (tipos view).
* **Validação** em TS com Zod/TypeBox **só** para view/inputs; “commit” para canon via WASM.

---

## 6) Segurança & Erros

* **Rust**: fonte da verdade de erros normativos (InvalidUTF8, NotNFC, NonMinimalVarint, …).
  TS **mapeia** para códigos/strings, **não** cria novos.
* **Assinaturas**: só Rust/WASM. **Nunca** JS puro.
* **Secrets**: HSM/KMS (server). Browser assina apenas com chaves de curta duração (se necessário) via WASM + WebCrypto como PRNG.

---

## 7) Testes & CI

* **Rust**: unit + integration + fuzz (BASE e MÓDULOS).
* **TS**: testes de orquestração/UI (Jest/Playwright), **chamando WASM** para validar canon/verify.
* **Cross**:

  * Roundtrip JSON(view)→WASM→canon→verify.
  * Offline bundle verification (TS+WASM).
  * Pipelines ATTEST→EVALUATE→TRANSACT com `pipeline_prev` (hash chain).

---

## 8) Versionamento & Distribuição

* **Canon/protocolos**: versionados por **CID (BLAKE3)** além de `vX.Y`.
* **WASM**: publicado como `@ubl/ai-nrf1-wasm@x.y.z`, **sempre** gerado do commit/tag do core.
* **SDKs TS**: dependem fixamente de major + verificação de compat com schema CID.

---

## 9) Exemplos de “quem faz o quê”

| Tarefa                                                    | Stack                                               |
| --------------------------------------------------------- | --------------------------------------------------- |
| Calcular `capsule_id`, assinar `seal`, verificar receipts | **Rust** (ou **TS via WASM**)                       |
| Transformar `ubl-json(view)` em canon                     | **Rust** (ou **TS via WASM**)                       |
| Renderizar status page, badge, ghost                      | **TS/React**                                        |
| Middleware que recibo API requests                        | **TS (Express/Next/Hono)** + WASM para `verify`     |
| Avaliar políticas                                         | **Rust/WASM guest**                                 |
| Orquestrar pipeline e consent k-of-n (fila)               | **Rust (prod serviço)**; TS pode ser edge/frontdoor |
| Gerar `product` a partir de manifesto                     | **CLI Rust** (codegen), TS só dispara comando       |

---

## 10) Layout de repositórios (sugestão)

```
/core
  ai-nrf1-core/         # Rust
  ubl-crypto/           # Rust
  ubl-capsule/          # Rust
  ai-json-nrf1-view/    # Rust
  cli/ubl/              # Rust

/bindings
  wasm/                 # wasm-pack, wrapper TS
  python/               # opcional: validação dif (não canônica)

/modules
  wrapper-template/     # Rust
  packs/                # JSON/TDLN/WASM guest

/products
  api-receipt-gateway/  # binário Rust
  ai-model-passport/    # binário Rust
  ...

/js
  packages/@ubl/sdk/
  packages/@ubl/verify/
  packages/@ubl/types/
  packages/@ubl/express-middleware-receipt/
  apps/status-page/     # Next.js
```

---

## 11) Decisões rápidas (FAQ)

* **“Preciso reimplementar canon em TS?”** Não. Use WASM.
* **“Posso assinar no browser?”** Sim, via WASM (e chaves efêmeras); preferir server/HSM.
* **“Posso fazer policies em TS?”** Não no hot-path. Policies = WASM guest ou Rust.
* **“CLI em TS?”** Pode ter *wrappers* TS que chamam o **binário Rust**. Sem lógica de canon no TS.

---

## 12) Próximos passos (execução)

1. **Congelar** crates Rust da BASE (tag + CID).
2. Gerar **`@ubl/ai-nrf1-wasm`** e **`@ubl/verify`**.
3. Subir **SDK** (`@ubl/sdk`) com exemplos `resolve(card_url)`, `verifyBundle`.
4. Publicar **status-page** (Next.js) usando WASM p/ verify offline.
5. Amarrar **API Receipt Gateway** end-to-end (Edge TS → fila → produto Rust → bundle → UI).

se quiser, eu já te entrego o esqueleto dos pacotes TS + bindings WASM e um `STACK-BOUNDARIES.md` pronto pra commit. só falar “manda scaffold” que eu já deixo no formato do repo. 🚀
