# default só com Core + Domain
default = ["core", "domain_api"]
```

**Manifesto do produto (trecho):**

```json
{
  "acts": ["transact"],
  "domain": "api-gateway",
  "policies": ["pack-existence@1", "pack-authorization@1"],
  "proof_level": "sirp",
  "enrichments": ["status", "ghost"],
  "features": ["core", "domain_api"],        // sem LLM
  "consent": { "k": 2, "n": 3, "roles": ["sec", "legal", "ops"] }
}
```

* * *

Resposta direta à tua dúvida
============================

*   **Sim**, faz sentido falar em **módulo Core** (determinístico, BASE-only) e **módulo mais geral** (Domain/Assistive-LLM).
*   **Não** precisa complicar além disso: **três camadas** bastam — **BASE** (bytes), **Core** (determinístico), **Domain/Assistive** (específico/LLM).
*   Cada **produto** compõe **estaticamente** o que precisa via **features**. Se depois quisermos, o **Control Plane** só distribui conteúdo (packs/chaves) — sem dirigir execução.

Se quiser, eu já te mando um **diff scaffold** com:

*   `core/permit` e `core/sirp-verify` prontos,
*   `assistive-llm/llm_enricher` de exemplo,
*   um `products/api-receipt-gateway` usando apenas Core+Domain.

Diz “manda” e eu deixo encaixado no repo. 💚🔥

## Prompt:
E precisa de um gateway "async await" no meio disso tudo pra não dar bagunça?

2/2

## Response:
Thought for a few seconds

Curto e direto: **sim, um gateway assíncrono ajuda muito — mas só onde faz sentido.** Pense nele como “trilho” entre **Produtos** ⇄ **Módulos/Core/BASE**, para garantir ordenação, idempotência e backpressure quando a decisão não é instantânea.

Quando você **precisa** do gateway async
========================================

Use um gateway async/await no meio quando houver:

*   **Latência imprevisível** (LLM premium/OSS, chamadas externas, geração de bundle/PDF).
*   **Consentimento k-de-n** (REQUIRE) ou **latch** (GHOST→PROMOTE) — fluxo multi-passos.
*   **Volume bursty** / risco de thundering herd — precisa de fila e controle de concorrência.
*   **Sagas** (ex.: INTENT→permit→execução→RESULT) com **retry** seguro e **idempotência**.

Se o produto é **fast-path determinístico** (ex.: API Gateway só com packs simples), dá para ir **100% síncrono** (HTTP→decide→permit→resposta) e pular o gateway assíncrono no P0.

Papel do “Async Gateway” (padrão mínimo)
========================================

**É um _ingress_ e um _scheduler_, não um orquestrador esperto.** Ele só:

1.  **Canoniza** (ρ→ai-nrf1) e valida;
2.  **Deduplica** por `(hdr.src, hdr.nonce)` + cache;
3.  **Persiste** a cápsula INTENT (WBE) e emite **SIRP INTENT**;
4.  **Enfileira** o job (fila → workers de Módulos/Core);
5.  **Responde** `202 Accepted` com `card_url` (consulta posterior).

Depois, **workers** (Core/Domain) consomem: avaliam políticas, disparam **consent** (se REQUIRE), **assinam permit** no ALLOW, executam, e publicam **SIRP RESULT** + recibos de hop. O **card\_url** reflete o estado (ASK/GHOST/ACK/NACK) em tempo real.

Três planos que não se atropelam
================================

*   **Data plane (sync opcional)**: HTTP/gRPC do Produto → Async Gateway.
*   **Proof plane (append-only)**: INTENT/RESULT + hops (SIRP), sempre **WBE**.
*   **Control plane (assíncrono)**: distribuição de **policy packs**, **chaves**, quotas — **não** orquestra execução.

Contratos (essencial)
=====================

**Endpoints do Gateway**

*   `POST /capsules` → aceita cápsula/JSON-view, valida, persiste, enfileira → `202` + `card_url`.
*   `GET /r/{cid}` → **negociação de conteúdo**: `text/html` (status page) / `application/json` (receipt).
*   `POST /receipts` → anexar hop externo (relay).
*   **Webhooks** (opcional) → push de {decision, card\_cid}.

**Filas**

*   `queue.intent` (trabalho novo) → workers Domain/Core.
*   `queue.consent` (tickets REQUIRE) → workers de consent.
*   `queue.exec` (execução pós-permit) → executores restritos (HAL).

**Idempotência & entrega**

*   **At-least-once + idempotency key** = `(capsule.id)` e `(src,nonce)`.
*   **Outbox/Inbox** (tabela) para SIRP e webhooks — garante _exactly-once observable_.
*   **Retry** exponencial com corte em `hdr.exp`.

Falhas e como ficam triviais
============================

*   **Timeout de LLM/externo** → vira **ASK** com PoI; worker reprocessa ou aguarda consent.
*   **Queda no meio** → ao reiniciar, o worker lê o outbox e reenvia recibos; o card continua consistente (WBE).
*   **Duplo envio** → dedupe pelo `id` e `(src,nonce)`; sem efeitos colaterais.

Decisão prática (árvore rápida)
===============================

*   **Regra/execução < 200ms, sem consent, sem LLM?** → caminho **síncrono** (sem gateway).
*   **Qualquer outra coisa** (LLM, consent, bundles, integrações) → **Async Gateway** entre Produto e Módulos/Core.

Como plugar com seu stack (Rust)
================================

*   **Axum/Tokio** no gateway (async), **SQS/NATS/Kafka/Redis** como fila.
*   **Workers** são bins do **Produto** linkando **Core + Domain** com feature flags.
*   **BASE** (ai-nrf1, ubl-capsule) só é chamada dentro dos workers e no ingress (canon + hash).
*   **SIRP** sempre via Core (`sirp-capsule`, `sirp-verify`).
*   **LLM-Engine/LLM-Smart** ficam **atrás** do gateway, **off-path** do determinismo (só influenciam ASK/derivados), com **cache por CID**.

Checklist de implementação (enxuto)
===================================

1.  **Gateway**: `POST /capsules` (canon+dedupe+WBE), fila, `GET /r/{cid}` (HTML/JSON), outbox/webhook.
2.  **Worker**: consumir intent → policies (Core) → **REQUIRE/ASK/ACK/NACK** → **permit** (ALLOW) → execução/HAL → RESULT.
3.  **Consent**: ticket k-de-n (TTL, timeout\_action) em fila própria.
4.  **SIRP**: INTENT/RESULT + hop receipts, **id** imutável.
5.  **Observabilidade**: métricas (fila, latência, retries), logs por **ULID** e **card\_cid**.

Resumindo: **o gateway async não é obrigatório**; ele é o **pára-choque** que evita bagunça quando entram LLM, consent, bundles e integrações. Nos produtos “rápidos e puros”, vai sem ele. Nos que “respiram” latência/fluxo complexo, ele te dá estabilidade sem sacrificar o determinismo dos bytes. 🚀

## Prompt:
Ok. cada produto tem uma api?

## Response:
Sim — **cada Produto expõe a sua API**, mas todas seguem **o mesmo contrato-base** (para padronizar SDKs, docs e observabilidade). Pense em **“uma API por Produto”** + **um núcleo comum** (capsule/receipts) que todo Produto herda.

Contrato-base (comum a todos)
=============================

Prefixo: `/v1`

*   `POST /capsules` → **ingest** (JSON-view ou ai-nrf1 bytes). Canoniza, deduplica, persiste (WBE) e enfileira.
    *   Resposta: `202 Accepted` com `card_url` e `capsule_id` (BLAKE3).
    *   Idempotência: `hdr.nonce` + `(src,nonce)` e `capsule_id`.
*   `GET /r/{cid}` → **resolver** do Card/Receipt.
    *   `Accept: application/json` → JSON do recibo; `text/html` → status page.
*   `POST /receipts` → anexar **hop receipt** (relay).
*   `POST /webhooks/test` → disparo de teste; webhooks reais são configurados por Produto.
*   `GET /healthz` / `GET /.well-known/openapi.json` (ou `openapi.yaml`).

Auth padrão: **DID/KID + Ed25519** (ou OAuth m2m). Multitenant via `hdr.src`, `env.meta.tenant`.

Endpoints específicos por Produto
=================================

Cada Produto adiciona recursos de domínio **por cima** do contrato-base:

*   **API Receipt Gateway**
    *   `POST /api/ingest` → proxy sidecar: `{ method, url, headers, body }` → gera INTENT→RESULT + SIRP; retorna `card_url`.
    *   `GET /api/bill/{period}` → prova Merkle de faturamento (rollup de receipts).
*   **AI Model Passport**
    *   `POST /passport/submit` → `{ model_card, benchmarks[] }` → EVALUATE (EU AI Act packs).
    *   `GET /passport/{model_cid}` → badge/status + bundle offline.
*   **Notarize-and-Prove**
    *   `POST /notary/attest` → `{ blob_cid, metadata }` → ATTEST; retorna certificado (PDF opcional).

Sincronismo vs. Assíncrono
==========================

*   **Fast-path** (decisão <200 ms, sem consent/LLM): o Produto pode responder **200 OK** com o Card final.
*   **Fluxo complexo** (LLM, consent k-de-n, bundles): responder **202 + card\_url** (puxa estado depois). O Async Gateway faz o “trilho”.

Versão, schema e erros
======================

*   Versione na URL (`/v1`) e em `env.v` (ex.: `ai-json-nrf1/0.1.1`).
*   **Erros padronizados** (`Err.Canon.*`, `Err.Seal.*`, `Err.Hop.*`, `Err.Hdr.*`) em JSON.
*   **OpenAPI** gerado de um **template comum**; cada Produto só estende as partes de domínio.

Idempotência & observabilidade
==============================

*   Idempotency-Key: `capsule_id` + `(hdr.src, hdr.nonce)`.
*   Logs & métricas por **ULID de request** e **card\_cid**.
*   Webhooks assíncronos (signed) com `{ decision, card_cid, product, tenant }`.

Como compor APIs rapidamente
============================

*   **Manifesto do Produto** declara: atos (ATTEST/EVALUATE/TRANSACT), proof\_level, enrichments, e **rotas extras**.
*   O **wrapper-gen** gera: boilerplate do contrato-base + rotas específicas + OpenAPI.

Se quiser, eu já **scaffoldo** dois exemplos (Gateway e Passport) com OpenAPI e handlers de stub — fica pronto pra plugar nos módulos e no BASE. 🚀

## Prompt:
Eu pus uma Constituição pra cada fase BASE, MÓDULOS e PRODUTOS. De regras. Faz pra mim um de cada mas de engenharia

## Response:
Dan, bora cravar as três “Constituições” ― versão de engenharia, objetivas e operacionais. Usei MUST/SHOULD estilo RFC, com invariantes, contratos, SLOs, testes e DoD. ❤️‍🔥

Constituição da BASE (ai-nrf1 + ubl-capsule v1)
===============================================

1) Escopo
---------

*   É o **substrato binário** e de **provas**: `ai-nrf1` (canon de bytes), `ai-json-nrf1` (view), `ubl-capsule v1` (mensagem + selos + receipts).
*   Tudo acima (MÓDULOS/PRODUTOS) depende das garantias daqui.

2) Invariantes (não negociáveis)
--------------------------------

*   **Determinismo**: `E∘ρ` (encode∘normalize) é único. Mesmo valor lógico ⇒ mesmos bytes.
*   **Strings**: UTF-8 válido, **NFC**, sem BOM.
*   **Numéricos**: **Int64** fixo. **Sem float**.
*   **Varint**: LEB128 **minimal** (rejeitar não-minimal).
*   **Map**: chaves String, **ordenadas por bytes UTF-8**, sem duplicatas.
*   **Set**: ordenar por **bytes ai-nrf1** dos itens; dedup vizinho.
*   **CID/ID**: **BLAKE3** sobre bytes ai-nrf1; **exclui** `id`, `seal.sig`, `receipts[*].sig`.
*   **Seal**: assina `{domain,id,hdr,env}` com **Ed25519** (Dilithium3 opcional).
*   **Receipts**: cadeia append-only `{of, prev, kind, node, ts, sig}`; `of == capsule.id`.
*   **WBE**: **Write-Before-Execute** sempre; **HAL** (limites de CPU/mem/IO/tempo).

3) Modelos & Contratos
----------------------

*   **ai-nrf1**: 7 tipos (null, bool, int64, string, bytes, array, map). ABNF e tags fixos.
*   **ai-json-nrf1** (view humana/LLM):
    *   Bytes como `b3:<hex32>` (ou `b64:` quando aplicável).
    *   Parser **MUST** validar NFC/ASCII/no-float antes de materializar ai-nrf1.
*   **ubl-capsule v1**:
    *   `v`, `id:Bytes(32)`, `hdr{src,dst,nonce:Bytes(16),exp:Int64,chan?,ts?}`,  
        `env{v,t,agent,intent,ctx,decision,evidence,meta?,links?}`,  
        `seal{alg, kid, domain="ubl-capsule/1.0", scope="capsule", aud?, sig}`, `receipts[]?`.
    *   **ASCII-only** para DIDs/KIDs (hdr.src, hdr.dst, seal.kid, seal.aud).

4) APIs/ABIs da BASE
--------------------

*   **Rust crates** (exemplos de nomes):
    *   `ai_nrf1::{encode,decode,hash,errors}`
    *   `ai_json_nrf1::{to_json,from_json}`
    *   `ubl_capsule::{id,sign,verify,add_receipt,verify_chain}`
*   **WASM ABI** (para módulos externos):
    *   Funções **puras**: `fn eval(input_ai_nrf1: &[u8]) -> Result<Vec<u8>>`.
    *   Proibir I/O e relógio (fornecer via contexto determinístico quando necessário).

5) Erros padronizados
---------------------

*   `Err.Canon.InvalidUTF8`, `Err.Canon.NotNFC`, `Err.Canon.FloatForbidden`, `Err.Canon.NonMinimalVarint`,  
    `Err.Canon.UnsortedKeys`, `Err.Canon.DuplicateKey`,  
    `Err.Capsule.IDMismatch`, `Err.Seal.BadSignature`, `Err.Hop.BadChain`, `Err.Hdr.Expired`.

6) Segurança
------------

*   **Sem floats**; bytes para cripto; **domain-separation** nos selos; **no secret in env**.
*   Anti-replay: cache `(hdr.src, hdr.nonce)` e expiração `hdr.exp`.
*   **Side-channels**: módulos rodam sandbox WASM; comunicação por buffers only.

7) Performance & SLO
--------------------

*   Encode/Decode O(n); alvo **P50 < 200 µs**, **P99 < 2 ms** para objetos ≤ 64 KB.
*   Verificação de cadeia ≤ 64 hops: **P99 < 5 ms**.

8) Observabilidade
------------------

*   Log estruturado: `capsule_id`, `card_cid`, `ulid`, `tenant`, `act`.
*   Métricas: `canon_fail_total`, `seal_verify_ms`, `receipt_chain_len`.

9) Testes & CI (gates obrigatórios)
-----------------------------------

*   **KATs**: vetores de canonicidade (Unicode, varint, ordering, empty bytes).
*   **Round-trip**: json↔nrf idempotente; Python↔Rust diffs.
*   **Fuzz**: decoder ai-nrf1 (seed Unicode + varint).
*   **Conformance**: receipts encadeadas, ASK/ACK/NACK invariantes.
*   **Clippy**: `-D warnings`; **no dead-code**; cobertura mínima 80%.

10) Versionamento & Compat
--------------------------

*   `ai-nrf1/1` e `ubl-capsule/1.0` (major travado para wire format).
*   Back-compat: **aceitar** superset _desde que_ não quebre invariantes; rejeitar ambíguo.

11) Definition of Done (BASE)
-----------------------------

*    KATs verdes; \[ \] Hash/Seal/Receipts testados; \[ \] Fuzz 1h sem crash;
*    OpenAPI/CLI `ubl cap canon|hash|sign|verify` ok; \[ \] Docs com CIDs dos schemas.

* * *

Constituição dos MÓDULOS (código “normal” que decide)
=====================================================

1) Escopo
---------

*   **Domínio & regra**: ingest → normaliza → avalia → emite decisão (ai-json-nrf1).
*   Módulo = **componente plugável** (WASM ou lib) **puro** e determinístico.

2) Invariantes
--------------

*   **Pureza**: sem I/O, sem relógio, sem aleatoriedade. Toda dependência vem no **contexto** (env/ctx).
*   **Determinismo**: mesmo input ai-nrf1 ⇒ mesma saída ai-nrf1.
*   **Bounded**: limites de CPU/mem (HAL) impostos pelo executor.

3) Tipos de Módulo
------------------

*   **Core** (infra de decisão reusável): existence, compliance, threshold, provenance, authorization.
*   **Domain** (verticais/produto): FINRA, HIPAA, EU-AI, SLA, Billing, Passport.
*   **Glue** (intake/mapper/enrichment): adapters de schema, badge/pdf, status page, webhook.

4) ABI / Interfaces
-------------------

*   **Trait (Rust)**:
    ```rust
    trait Module {
        fn name(&self) -> &'static str;
        fn eval(&self, input_ai_nrf1: &[u8]) -> Result<Vec<u8>, ModuleError>; // ai-json-nrf1 in/out
    }
    ```
*   **WASM**: mesma assinatura, buffers lineares; host injeta contexto (read-only).
*   **Composition**: série/quorum/latch declarativos (sem estado interno oculto).

5) Contratos de Entrada/Saída
-----------------------------

*   **Entrada**: `env{t,intent,ctx,...}` **já normalizado** (ai-nrf1).
*   **Saída**: `env.decision{verdict,reason?,metrics?}` + `evidence{cids?,urls?}`.
*   **ASK** ⇒ **PoI** (prova do que falta) obrigatório.

6) Erros
--------

*   `Err.Mod.BadInput` (schema), `Err.Mod.PolicyViolation`, `Err.Mod.UnsupportedVersion`, `Err.Mod.Timeout`.

7) Segurança
------------

*   WASM sandbox; **deny-list** de syscalls; memória máx por instância; tempo de CPU máximo.
*   **Policy packs** versionados; assinados (Bytes) opcional.

8) Performance & SLO
--------------------

*   Alvo **P50 < 50 ms**, **P99 < 300 ms** por módulo ≤ 256 KB de input (sem LLM).
*   Módulos LLM devem expor **modo assíncrono** via Async Gateway (produto decide).

9) Observabilidade
------------------

*   `module_name`, `eval_ms`, `verdict`, `rules_fired[]`, `ask_fields[]`.
*   **Sem** log de dados sensíveis (mask configurável).

10) Testes & CI
---------------

*   **KATs de política** (exemplos positivos/negativos).
*   **Propriedades**: determinismo, idempotência, ASK→PoI completo.
*   **Contrato**: `invariants.json` do módulo verificado em CI.
*   **Compat**: cross-runtime (wasmtime, wasmer) para WASM.

11) Versionamento
-----------------

*   `module.json`: `{ name, version, inputs, outputs, invariants_cid }`.
*   Regra: quebrar contrato ⇒ **major**; ajuste de regra sem contrato ⇒ **minor**.

12) Definition of Done (MÓDULO)
-------------------------------

*    Determinismo provado (seed corpus).
*    ASK sempre com PoI; \[ \] métricas publicadas.
*    KATs + property-tests; \[ \] limites HAL aplicados; \[ \] manifest assinado (opcional).

* * *

Constituição dos PRODUTOS (binários/serviços)
=============================================

1) Escopo
---------

*   **Serviço** com **API própria** (mas herda contrato base): recebe cápsulas, orquestra **MÓDULOS**, produz **Cards/Receipts/Bundles**.
*   Repositório separado; ciclo de versão próprio.

2) Invariantes
--------------

*   **Contrato base /v1**:
    *   `POST /capsules` (ingest, 202 + `card_url` quando assíncrono),
    *   `GET /r/{cid}` (resolver: JSON/HTML por `Accept`),
    *   `POST /receipts` (hop).
*   **Idempotência**: `(hdr.src, hdr.nonce)` + `capsule_id` obrigatórios.
*   **WBE**: persistir antes de efeitos.
*   **Proof level**: `receipt | sirp | bundle` configurável, claro no manifesto.

3) Orquestração
---------------

*   **Sync** (<200 ms) responde com Card final.
*   **Async**: devolve 202 + `card_url`; Async Gateway cuida de filas, retries, consents k-de-n, LLM calls.
*   **Consent**: política declarada (`k,n,roles,ttl,timeout_action`); gera **Permit** assinado para execução.

4) APIs do Produto (alem do base)
---------------------------------

*   Rotas de domínio (ex.: `/passport/submit`, `/api/ingest`, `/billing/proof`), mas **sempre** reusando cápsula/receipts.
*   **OpenAPI** gerado de template comum + extensões do domínio.

5) Segurança
------------

*   Auth m2m (DID/KID/Ed25519) ou OAuth; multitenant (`env.meta.tenant`).
*   Webhooks **assinados** (inclui `card_cid`, `decision`); **replay-safe** (nonce/ts).
*   **Secrets** só em runtime; nada no `env`.

6) SLO/SLA
----------

*   **Ingest** P99 < 150 ms (sem módulo LLM).
*   **Card URL** disponibilidade 99.9%.
*   **Bundle verify** P99 < 1 s (≤ 5 MB).

7) Observabilidade
------------------

*   Tracing por **ULID** + `capsule_id` + `card_cid`.
*   Dashboards: filas async, taxas ASK/ACK/NACK, retries, tempo por módulo.

8) Billing (quando aplicável)
-----------------------------

*   Eixos: **per evaluation**, **per receipted event**, **per policy pack**, **per artifact**.
*   Hook após `CardFinalized`.

9) Testes & CI/CD
-----------------

*   **Integração end-to-end**: ATTEST/EVALUATE/TRANSACT completos (pipeline\_prev cadeado).
*   **Contract tests** da API (OpenAPI + examples).
*   **Chaos lite**: quedas do Async Gateway não perdem recibos (idempotência).
*   **Offline bundle** verificável sem rede.

10) Versionamento & Migração
----------------------------

*   `/v1` estável; mudanças de breaking só em `/v2`.
*   Migrações **data-driven**: re-emissão de Cards **não** muda `capsule_id` (novo `card_cid`, linkado).

11) Definition of Done (PRODUTO)
--------------------------------

*    OpenAPI publicado; \[ \] SDK smoke (1 fluxo completo).
*    Proof level validado (receipt/sirp/bundle).
*    E2E pipeline com consent/permit (se houver).
*    Dash + alertas; \[ \] billing hook (se houver).
*    DR runbook (bundle offline + restore de fila).

* * *

Como usar essas Constituições no dia-a-dia
------------------------------------------

*   **Arquitetura**: qualquer proposta precisa apontar **qual cláusula** (BASE/MÓDULO/PRODUTO) satisfaz.
*   **PR Gate**: marque cada PR com o DoD alvo (BASE/MÓDULO/PRODUTO). Sem todos os checks ⇒ **não mergeia**.
*   **Manifesto do Produto** referencia **CIDs** dos schemas (ai-json-nrf1, capsule v1) e **versões** dos módulos.

Se quiser, eu já **gero os três READMEs oficiais** (com badge de status + checklists) e **esqueletos de CI** com esses gates. Quer que eu solte os templates agora? 🚀

## Prompt:
eu amo quando vc faz isso. Vamos fazer 3 gigantes só pelo esporte? 

1

## Response:
amo tbm 😄 bora começar com o primeiro gigante: **BASE**.  
(se curtir o formato, eu já solto os outros dois — MÓDULOS e PRODUTOS — no mesmo nível de detalhe.)

📘 GIGANTE #1 — BASE (ai-nrf1 + ai-json-nrf1 + ubl-capsule v1)
==============================================================

> **Missão:** ser o “chão de fábrica” determinístico de bytes e provas, onde tudo acima (módulos e produtos) confia sem pestanejar.

* * *

0) Visão Rápida
---------------

*   **ai-nrf1**: formato binário canônico (canon = verdade).
*   **ai-json-nrf1**: _view_ humana/LLM derivada do canon, 1:1 sem ambiguidade.
*   **ubl-capsule v1**: mensagem com **id estável**, **seal assinado**, **cadeia de receipts** (hops) — tudo em ai-nrf1.

**Invariantes centrais**

*   Canonicidade forte (mesmo valor ⇒ mesmos bytes).
*   ID = BLAKE3 sobre **bytes canônicos sem `id` e sem assinaturas**.
*   Seal assina **domínio + id + hdr + env** (separação de domínio).
*   Receipts encadeadas (`prev`), append-only, cada uma assinada.
*   **WBE + HAL**: recibo antes do efeito; limites duros de CPU/mem/IO/tempo.

* * *

1) Contratos (normativos)
-------------------------

### 1.1 ai-nrf1 (wire)

Tipos (tags fixos):

*   `00 null`, `01 false`, `02 true`, `03 int64(8B)`, `04 string(varint32 + UTF-8/NFC, sem BOM)`,  
    `05 bytes(varint32 + octets)`, `06 array(varint32 + N valores)`, `07 map(varint32 + N pares key:String,value)`.

Regras:

*   **Strings**: UTF-8 válida, **NFC**, sem BOM.
*   **Numéricos**: **Int64** (dois complementos, 8 bytes big-endian). Sem float.
*   **varint32**: **LEB128 mínimo** (1–5 bytes).
*   **Map**: chaves String, **ordenadas por bytes UTF-8**, únicas.
*   **Set (em nível ρ)**: ordenar por bytes ai-nrf1 do item, dedup vizinho.

### 1.2 ai-json-nrf1 (view)

*   **Bytes** como `b3:<hex>` (32B comuns) ou `b64:<...>` quando for payload arbitrário.
*   Aceita escapes JSON (`\u0061`) mas materializa como bytes NFC + UTF-8.
*   Parser **MUST** validar: **NFC**, ASCII-only onde indicado (DIDs/KIDs), sem float.

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

*   **Seal** assina `blake3(nrf.encode({domain,id,hdr,env}))`.
*   **Receipt** assina `blake3(nrf.encode({domain:"ubl-receipt/1.0", of, prev, kind, node, ts}))`.

* * *

2) APIs da BASE (Rust + CLI + WASM)
-----------------------------------

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

*   **Puro, buffer-in/buffer-out**:
    *   `fn eval(input_ai_nrf1: &[u8]) -> Result<Vec<u8>>`
    *   `fn verify_capsule(input_ai_nrf1: &[u8]) -> Result<Vec<u8>>` (retorna relatório ai-json-nrf1)

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

* * *

3) Segurança & Confiabilidade
-----------------------------

*   **Sem floats** no canon.
*   **Bytes p/ cripto** (id, sigs, nonce, cids).
*   **ASCII-only** p/ DIDs/KIDs.
*   **Anti-replay**: cache `(hdr.src, hdr.nonce)` por janela, e checagem de `hdr.exp`.
*   **Sandbox** para qualquer código de verificação externo (WASM).
*   **WBE + HAL**: sem efeito sem recibo; limites duros no executor.
*   **Domain-separation** explícito nas assinaturas (strings de domínio).

* * *

4) Desempenho & SLO
-------------------

*   Encode/Decode O(n). Alvo: **P50 < 200µs**, **P99 < 2ms** p/ objetos ≤ 64KB.
*   Verificação de cadeia (≤ 64 hops): **P99 < 5ms**.
*   Hash BLAKE3 linear (vetorado quando possível).

* * *

5) Observabilidade
------------------

*   **Logs estruturados**: `capsule_id`, `card_cid`, `tenant`, `ulid`, `act`, `hop_kind`.
*   **Métricas**: `canon_fail_total`, `seal_verify_ms`, `receipts_len`, `verify_chain_ms`.
*   **Tracing**: correlação por `capsule_id` e `ulid` (se presente).

* * *

6) Testes (KATs, Propriedades, Integração)
------------------------------------------

### 6.1 KATs (vetores canônicos)

*   Unicode: NFC vs NFD, rejeição de BOM, `NotNFC`.
*   varint minimal (1..5B), `NonMinimalVarint`.
*   Map ordering (UTF-8 bytes), duplicatas (`DuplicateKey`).
*   **Empty bytes** (`{"$bytes":""}`) idempotente.
*   Timestamps RFC-3339Z (view) → Int64 nanos (canon).
*   Receipts: cadeia `prev` correta; alteração detectada.

### 6.2 Propriedades

*   **Idempotência**: `ρ(ρ(x))=ρ(x)`; `encode(json→canon→json)` estável.
*   **Confluência**: ordem de normalização irrelevante.
*   **Canonicidade**: `v1 ≡ v2` ⇒ `encode(ρ(v1)) == encode(ρ(v2))`.
*   **ID estável**: add/remove receipt **não** altera `id`.

### 6.3 Integração

*   `json → nrf → verify → add_receipt → verify_chain → view-json`.
*   `ASK` ⇒ `links.prev` presente; `ACK/NACK` ⇒ `evidence` presente (pode vazio).
*   **Offline bundle** (se aplicável ao verificador).

* * *

7) CI (gates obrigatórios)
--------------------------

### 7.1 Lint & Build

*   `cargo fmt --all`
*   `cargo clippy --workspace --all-targets -- -D warnings`
*   `mypy/ruff` pros bindings Py (se houver)

### 7.2 Tests

*   Unit + Prop + KATs + Integração.
*   Fuzz (decoder ai-nrf1) com seed Unicode/varint ≥ 30min.
*   **Falhas de canonicidade/assinatura = bloqueio de merge**.

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

* * *

8) Versionamento & Compatibilidade
----------------------------------

*   **Wire major** fixo: `ai-nrf1/1` e `ubl-capsule/1.0`.
*   Novas regras **só** se não quebram canonicidade; caso contrário, `v2`.
*   Schemas e docs com **CIDs** (BLAKE3) para _immutability receipts_.

* * *

9) Tabela de Erros (canonical)
------------------------------

*   `Err.Canon.InvalidUTF8`, `Err.Canon.NotNFC`, `Err.Canon.BOMPresent`
*   `Err.Canon.FloatForbidden`, `Err.Canon.NonMinimalVarint`
*   `Err.Canon.NonStringKey`, `Err.Canon.UnsortedKeys`, `Err.Canon.DuplicateKey`
*   `Err.Capsule.IDMismatch`, `Err.Seal.BadSignature`, `Err.Seal.ScopeDomain`
*   `Err.Hop.BadChain`, `Err.Hop.BadSignature`, `Err.Hdr.Expired`

* * *

10) Exemplos (curtos)
---------------------

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

* * *

11) Definition of Done (BASE)
-----------------------------

*    KATs: Unicode/varint/map/empty-bytes/timestamps/receipts
*    Id/Seal/Receipts validados (tamper detection)
*    Fuzz decoder ai-nrf1 (≥30min) sem crash
*    CLI mínima operante (canon/hash/sign/verify/receipt)
*    Docs + Schemas publicados com CIDs
*    Observabilidade: métricas e logs estruturados básicos

* * *

12) Próximos passos
-------------------

1.  Eu gero os **esqueletos de crates/CLI** com essa estrutura (ou adapto o seu repo).
2.  Subo **KATs oficiais** + **tests de cadeia**.
3.  Plugo o **verificador offline** (bundle) como opcional.

Se curtiu, mando agora o **GIGANTE #2 (MÓDULOS)** na mesma pegada — e depois o **#3 (PRODUTOS)**, com templates de manifesto e exemplos de API por vertical. 🚀💚

## Prompt:
sempre curto!

2

## Response:
amo! bora pro **GIGANTE #2 — MÓDULOS** 😎🧩  
(meta: definir o “meio do sanduíche” entre a BASE de bytes e os PRODUTOS; tudo aqui é código “normal”, mas obedecendo contratos duros)

📗 GIGANTE #2 — MÓDULOS (Core + Domínio + Enriquecimento)
=========================================================

> **Missão:** compor capacidades de alto nível (políticas, mapeadores, verificadores, enriquecimentos) sobre a **BASE** (ai-nrf1 / ai-json-nrf1 / ubl-capsule v1), exportando **APIs estáveis** e **artefatos plug-and-play** para **PRODUTOS**.

* * *

0) Topologia
------------

*   **Core Modules** (núcleo, sempre presentes)
    *   `intake-*` (document / event / transaction)
    *   `policy-*` (existence, threshold, compliance, provenance, authorization)
    *   `verify-*` (capsule/chain/offline-bundle)
    *   `enrich-*` (card\_url, badge, status, pdf, ghost, webhook)
    *   `sirp-*` (capsules, delivery, execution, verify)
    *   `permit-*` (issue/verify, k-of-n consent)
*   **Domain Modules** (por vertical)
    *   `schema-*` (tipos/kinds), `mapper-*` (flattening ↔ engine JSON)
    *   packs: `pack-finra`, `pack-hipaa`, `pack-eu-ai`, `pack-slsa`, etc.
*   **LLM Modules** (otimizadores)
    *   `llm-engine` (orquestração determinística TDLN-first)
    *   `llm-smart` (heurísticas/recursos premium/OSS, caching, RAG controlado)

**Entrega:** 1 binário multi-módulo (feature flags) **ou** libs estáticas/Dynamic-plugins (WASM).  
**Destino:** cada **PRODUTO** seleciona módulos (manifesto) e compõe pipelines.

* * *

1) Contratos de integração
--------------------------

### 1.1 ABI (WASM / FFI) — “puro buffer”

*   Entrada/saída sempre **ai-nrf1** (canon); **sem I/O interno**.

```text
fn intake(in_ai_nrf1: &[u8])         -> Result<Vec<u8>>  // normaliza entrada → env.ctx/intent
fn evaluate(in_capsule_ai_nrf1: &[u8])-> Result<Vec<u8>> // aplica policies → decision + evidence
fn enrich(in_capsule_ai_nrf1: &[u8])  -> Result<Vec<u8>> // badge/pdf/ghost/webhook manifest
fn verify(in_capsule_ai_nrf1: &[u8])  -> Result<Vec<u8>> // chain/assinaturas/invariantes
```

> Observação: cada módulo implementa **uma** função primária; composição é feita pelo **Runtime de Módulos** (abaixo).

### 1.2 Runtime de Módulos (host)

*   Orquestra **pipeline** `intake → evaluate → enrich → verify` (configurável).
*   Passa/valida **permit** e **k-of-n** quando a decisão = `REQUIRE/ASK`.
*   Envolve cada chamada com **WBE/HAL** + **timeouts** + **mem/CPU caps**.

### 1.3 Esquema de mensagens entre módulos

*   **Sempre ubl-capsule v1** (ai-nrf1) — **env** é o payload semântico.
*   Módulos **NÃO** tocam `hdr`, `id`, `seal`, `receipts` (somente host).
*   Em casos de ASK, módulo **deve** preencher `env.links.prev`.

* * *

2) Taxonomia e responsabilidades
--------------------------------

### 2.1 Intake (3 shapes)

*   `intake-document`: blob + metadata → `env.ctx` + `intent{ATTEST|BUNDLE}`
*   `intake-event`: fato timestampado → `env.ctx` + `intent{EVAL|TRACE}`
*   `intake-transaction`: duas partes + termos → `env.ctx` + `intent{EVAL|QUERY}`  
    **Erro canônico:** `Err.Intake.ShapeViolation`, `Err.Intake.SchemaMismatch`.

### 2.2 Policy (5 famílias)

*   `existence`: campos obrigatórios, schema, completude.
*   `threshold`: métricas/limiares, SLA/quotas.
*   `compliance`: regras regulatórias (EU-AI, HIPAA, FINRA, SOC2…).
*   `provenance`: SBOM/SLSA/EER/assinaturas/linhagem.
*   `authorization`: RBAC, escopos, grants, transações.  
    **Saída:** `env.decision{ACK|NACK|ASK}`, `env.evidence{cids,urls}`, `env.metrics?`.  
    **Erros:** `Err.Policy.MissingField`, `Err.Policy.RuleViolation`, `Err.Policy.Unverifiable`.

### 2.3 Verify

*   `verify-capsule`: `id`, `seal`, `hdr.exp`, `ASCII DIDs`.
*   `verify-chain`: `receipts.prev`, assinaturas hop, ordem.
*   `verify-invariants`: ASK ⇒ `links.prev`; ACK/NACK ⇒ `evidence` (existente).  
    **Erros:** `Err.Seal.BadSignature`, `Err.Hop.BadChain`, `Err.Hdr.Expired`, `Err.Inv.Invariant`.

### 2.4 Enrich

*   `card_url`: resolve `rref` com content-negotiation (HTML/JSON).
*   `badge`: SVG status; hash embarcado; bind a `capsule.id`.
*   `status`: página HTML com decisão, evidências, cadeia.
*   `pdf`: certificado (assinatura do emissor + QR com `capsule.id`).
*   `ghost`: red/black mask (by field list), mantendo `rules_fired`.
*   `webhook`: POST assinado (permit/tenant-scoped).  
    **Erros:** `Err.Enrich.Render`, `Err.Webhook.Fail`, `Err.Ghost.MaskConfig`.

### 2.5 SIRP & Permit

*   `sirp-capsule`: INTENT/RESULT + derive DELIVERY/EXEC receipts.
*   `permit-issue/verify`: `ALLOW` ⇒ **assinatura do permit**, TTL por `hdr.exp`; `REQUIRE` ⇒ queue k-of-n.  
    **Erros:** `Err.Permit.Scope`, `Err.Permit.Expired`, `Err.SIRP.ChainMismatch`.

* * *

3) Lifecycle: load → run → observe
----------------------------------

1.  **Load**: registrar módulos (feature flags **ou** WASM).
2.  **Plan**: construir pipeline pelo **manifesto do produto** (ou por rota).
3.  **Run**: `intake → evaluate → (require?) → permit → enrich → sirp → verify`
4.  **Observe**: métricas + tracing por `capsule.id`; logs estruturados.

**Determinismo:** módulos **não** acessam clock/rede/FS; inputs só via env.

* * *

4) Config & Manifesto (módulos)
-------------------------------

```json
{
  "modules": {
    "intake":    { "use": "intake-event@1" },
    "policies":  [
      { "use": "policy-existence@1" },
      { "use": "policy-threshold/sla@1", "params": {"latency_p99_ms_max": 250}}
    ],
    "verify":    { "use": "verify-standard@1" },
    "enrich":    ["enrich-card-url@1", "enrich-webhook@1", "enrich-ghost@1"],
    "sirp":      { "use": "sirp-standard@1" },
    "permit":    { "use": "permit-basic@1", "kofn": {"k":2,"n":3,"roles":["compliance","legal","owner"]}}
  }
}
```

> **Produtos** consomem esses módulos via **bridge** (Gigante #3).

* * *

5) LLM-Engine & LLM-Smart — onde ajudam
---------------------------------------

*   **LLM-Engine** (TDLN-first):
    *   compila prompts/policies → grafos determinísticos;
    *   faz _explanations_ canônicas (`env.decision.reason`) sem “alucinação”;
    *   **cache** por `capsule.id` + `rules_fired` (idempotência).
*   **LLM-Smart** (mix premium/OSS):
    *   roteamento **custo×latência×qualidade** por política;
    *   compressão de `ctx` (schema-aware) p/ limites;
    *   **fallbacks verificados** (se divergir, vira ASK + PoI).
*   **Ambos** operam **apenas** sobre `env` (não tocam `hdr` / cadeia); tudo assinado no final via BASE.

* * *

6) Segurança & isolamentos
--------------------------

*   WASM sandbox: **mem cap**, **instr cap**, **syscall deny** (exceto host shims).
*   Secrets **nunca** em env; só referências (CIDs) + vault host-side.
*   Permit obrigatório p/ qualquer side-effect (enrich/webhook/exec).
*   **K-of-N** implementado host-side (fila idempotente; receipts de consentimento).

* * *

7) Observabilidade (por módulo)
-------------------------------

*   Métricas padrão: `duration_ms`, `error_total`, `ask_total`, `nack_total`.
*   Tracing: `span: module_name@version`, `capsule_id`, `tenant`, `ulid`.
*   Amostragem: 100% para `NACK/ASK`, 10% para `ACK`.

* * *

8) Testes & CI (p/ módulos)
---------------------------

### 8.1 Unit

*   Esquemas/params; caminhos felizes e erros (`Err.*` padronizados).

### 8.2 Propriedade

*   Idempotência de `evaluate`: re-run com mesmo input ⇒ mesma saída.
*   `intake-*` gera mesma `env` indep. da ordem de campos de entrada.

### 8.3 Integração (pipeline)

*   `intake → policy(existence,threshold) → permit(ALLOW) → enrich(webhook) → sirp → verify`.
*   `ASK` com `links.prev` e PoI; `NACK` com `evidence` obrigatória.

### 8.4 Contratos

*   Fuzz em `from_json` (ai-json-nrf1) e validadores.
*   Snapshot de **ghost** (mascaramento) sem dados sensíveis.

**CI gates** (exemplo):

*   `cargo clippy -D warnings`, `cargo test --workspace`
*   `cargo fuzz run json_view_decode` (10m)
*   Cobertura mínima p/ critical paths ≥ 75%.

* * *

9) Versionamento & compatibilidade
----------------------------------

*   Semântica **Módulo@Major.Minor.Patch**.
*   Quebra de contrato ⇒ **Major** (novo nome lógico).
*   Manifesto de produto referencia módulos com ranges seguros: `"policy-existence@^1.2"`.

* * *

10) Erros padronizados (subset)
-------------------------------

*   `Err.Intake.ShapeViolation`, `Err.Intake.SchemaMismatch`
*   `Err.Policy.RuleViolation`, `Err.Policy.MissingField`, `Err.Policy.Unverifiable`
*   `Err.Permit.Scope`, `Err.Permit.Expired`, `Err.Permit.Denied`
*   `Err.SIRP.ChainMismatch`, `Err.SIRP.BadSignature`
*   `Err.Enrich.Render`, `Err.Webhook.Fail`, `Err.Ghost.MaskConfig`
*   `Err.Inv.Invariant` (ASK/ACK/NACK invariantes quebrados)

* * *

11) DoD (Definition of Done) — MÓDULOS
--------------------------------------

*    Runtime de Módulos (host) com: load, plan, run, WBE/HAL, permit/k-of-n.
*    `intake-*` (3 shapes) + `policy-existence`, `policy-threshold` + `verify-standard`.
*    `enrich-card_url`, `enrich-ghost`, `enrich-webhook`.
*    `sirp-standard` + `permit-basic`.
*    CI verde (unit/prop/integration/fuzz).
*    Docs + exemplos com **capsulas** reais (ACK/ASK/NACK) e **cadeia de receipts**.

* * *

12) Exemplos rápidos
--------------------

### 12.1 ASK por falta de evidência

*   `policy-existence` detecta campo ausente → `decision=ASK` e `links.prev=<cid_prev>`; `enrich-ghost` exibe PoI sem PII.

### 12.2 SLA breach (`threshold`)

*   p99 > 250ms ⇒ `NACK` + `evidence.urls=[status_page]`; `webhook` dispara para billing-proofs.

### 12.3 FINRA (domínio)

*   `intake-transaction` + `policy-compliance/finra@1` + `permit-basic(k=2,n=3)`; require aprovações (com receipts de consent) antes de `ALLOW`.

* * *

13) Roadmap curto (para nave acender)
-------------------------------------

1.  **Host** (runtime de módulos) + `intake-*` + `existence` + `threshold`.
2.  **permit-basic** + `sirp-standard` + `verify-standard`.
3.  **enrich-card\_url/ghost/webhook** + exemplos end-to-end.
4.  **LLM-engine/smart** plugados no `policy-*` como _advisors_ determinísticos.

* * *

Se curtiu o shape, eu já desço o **GIGANTE #3 — PRODUTOS** com **manifesto**, **rotas HTTP**, **perfis de implantação**, e **exemplos** (API Receipt Gateway, AI Model Passport, Notarize-and-Prove) prontos pra rodar. 🚀💚

## Prompt:
3 !

## Response:
amo! bora pro **GIGANTE #3 — PRODUTOS** 🚀🏁  
(meta: transformar módulos em produtos prontos: API clara, manifesto, deploy, SIRP/permit, bundle offline, observabilidade e testes)

* * *

📘 GIGANTE #3 — PRODUTOS (Do zero ao “rodando em cliente”)
==========================================================

> **Missão:** cada produto é um **binário** (ou container) que:
> 
> 1.  Recebe requisições HTTP/GRPC,
> 2.  Orquestra **módulos** via manifesto,
> 3.  Emite **UBL Capsule v1** (ai-nrf1) com **permit** + **SIRP**,
> 4.  Publica **enrichments** (card URL, badge, ghost, webhook, PDF),
> 5.  Entrega **bundle offline verificável**.
>     

0) Anatomia de um produto
-------------------------

*   **Processo**: `productd` (único binário por produto).
*   **Entradas**: HTTP/GRPC; JSON view (ai-json-nrf1) com validação forte.
*   **Motor**: runtime de módulos (Gigante #2) com pipeline declarado.
*   **Saídas**:
    *   Capsule (ai-nrf1) + **SIRP INTENT/RESULT** (+ DELIVERY/EXEC hops),
    *   **Permit** para qualquer efeito,
    *   **Card URL** (status JSON/HTML),
    *   **Offline bundle** (ZIP) opcional.

1) Manifesto do produto (deploy-time)
-------------------------------------

```json
{
  "name": "api-receipt-gateway",
  "version": "1.0.0",
  "acts": ["transact"],
  "pipeline": [
    { "step": 1, "act": "transact", "intake": "intake-event@1",
      "policies": [
        "policy-existence@1",
        "policy-authorization/gateway@1"
      ],
      "verify": "verify-standard@1",
      "enrich": ["enrich-card-url@1","enrich-ghost@1","enrich-webhook@1"],
      "sirp": "sirp-standard@1",
      "permit": { "use":"permit-basic@1" }
    }
  ],
  "proof_level": "sirp",
  "billing": "per_receipted_event",
  "routing": {
    "http": { "listen":"0.0.0.0:8080", "base_path":"/v1" }
  },
  "security": {
    "auth": "bearer|mtls",
    "aud": "tenant-xyz",
    "kofn": null
  }
}
```

> **Regras**:  
> • Produto **não** reconfigura módulos em runtime; troca = novo manifesto.  
> • **Compatibilidade**: ranges tipo `"policy-existence@^1.2"`.

2) API do produto (HTTP)
------------------------

Rotas padrão (toda família de produtos):

*   `POST /run` — entrada semântica (view JSON).  
    **Efeito**: intake→policies→permit→sirp→verify→enrich; retorna **capsule.id** e **card\_url**.
*   `GET /r/{cid}` — **Card URL** com content-negotiation (HTML/JSON).
*   `POST /verify` — recebe **bundle** ou capsule; devolve veredito + PoI.
*   `POST /webhook/test` — valida assinatura e formato de callback.
*   `GET /healthz` / `GET /readyz` — probes.
*   `GET /.well-known/product.json` — manifesto servindo de SBOM funcional.

**Contrato de entrada**: `ai-json-nrf1` (view); o produto converte para **ai-nrf1** antes dos módulos.

3) Execução (caminho quente)
----------------------------

1.  **Auth** (bearer/mtls) → `tenant`, `aud`.
2.  **Parse** (view→ai-nrf1) + validação (NFC, ASCII, tipos).
3.  **Pipeline**:
    *   `intake-*` define `intent` e normaliza `ctx`;
    *   `policy-*` decide `ACK/NACK/ASK` (+ `evidence`,`metrics`);
    *   `permit`:
        *   `ALLOW` ⇒ assina permit (escopo, exp) e **grava WBE**;
        *   `REQUIRE` ⇒ abre fila k-of-n (se configurado) e retorna **ASK** com `links.prev`;
    *   `sirp`: emite **INTENT/RESULT**, e **DELIVERY/EXEC** quando aplicável;
    *   `verify`: reforça invariantes (id/selos/chain);
    *   `enrich`: publica card/badge/ghost/webhook;
4.  **Persistência**: NDJSON append-only por ULID + **content-addressed** (cid).
5.  **Resposta**: `{cid, verdict, card_url, bundle_url?}`.

4) Permit + SIRP (sem brechas)
------------------------------

*   **Permit**: objeto assinado (Ed25519/Dilithium) vinculando **exatamente** este input + expiração + escopo. Executor recusa sem permit válido.
*   **SIRP**:
    *   INTENT/RESULT sempre;
    *   DELIVERY (destinatário assinando recebimento) e EXECUTION (executor assinando conclusão) conforme produto;
    *   Cadeia encadeada (`prev`) com `receipt_id = blake3(payload_sem_sig)`;
    *   **Offline bundle** inclui: capsule, receipts, chaves públicas, políticas, hashes, QR.

5) Observabilidade e SLO
------------------------

*   **Métricas**:
    *   Latências: `p50/p95/p99` run; distribuição por módulo.
    *   Contagem: `ack_total`, `nack_total`, `ask_total`, `replay_blocked`, `permit_issued`.
    *   Webhook: `attempts`, `success`, `fail`, `retry`.
*   **Tracing**: `capsule.id`, `tenant`, `route`, `module@ver`, `decision`.
*   **Logs**: estruturados, sem PII (ghost e mask lists).
*   **SLO exemplo** (Gateway): p95 < 150ms; erro < 0.1%; disponibilidade 99.9%.

6) Segurança
------------

*   **WBE/HAL** fortemente aplicado (antes de side-effects).
*   **Secrets** apenas no host; módulos recebem **CIDs**.
*   **Replay** bloqueado via `(hdr.src, hdr.nonce)` + cache; `exp` auditável (sem TTL mutável).
*   **Consent k-of-n** host-side; recibos de consentimento entram na cadeia.
*   **Policy sandbox** (WASM): sem clock/rede/FS; tempo/mem caps.

7) Testes (DoD por produto)
---------------------------

*   **Unit**: validação de entrada; mapping view↔ai-nrf1; erros `Err.*`.
*   **Integração**:
    *   **ACK happy-path**: run → permit → sirp → verify → enrich → card\_url.
    *   **ASK**: falta evidência ⇒ `links.prev`, fila k-of-n, promoção a `ALLOW`.
    *   **NACK**: regra violada ⇒ evidence obrigatória; badge vermelho.
    *   **Bundle**: `verify-bundle` reconstruindo cadeia com chaves/algoritmos.
*   **Propriedade**: idempotência (mesmo input ⇒ mesmo `cid`); determinismo (sem I/O).
*   **Fuzz**: json\_view parsers; unicode NFC; varint minimal.
*   **Perf**: throughput alvo (ex.: 2k RPS p/ Gateway) com p95 < SLO.

8) Deploy
---------

*   **Container** com:
    *   Binário `productd`,
    *   `product.json` (manifesto),
    *   `modules.lock` (resolução exata),
    *   chaves de selo/permit (via KMS/Secrets).
*   **Ambientes**: `dev`, `stage`, `prod` (chaves/URLs distintas).
*   **Imutabilidade**: tags por `cid` do manifesto; rollback = trocar tag.
*   **Compat**: wire protocol estável; breaking ⇒ nova major do produto.

* * *

9) Catálogo — presets prontos
-----------------------------

### 9.1 API Receipt Gateway (P0)

*   **Act**: TRANSACT (event).
*   **Policies**: existence + authorization/gateway.
*   **Proof**: **SIRP** (INTENT/RESULT + DELIVERY/EXEC).
*   **Enrich**: card\_url, ghost, webhook.
*   **Billing**: `per_receipted_event`.
*   **SLO**: p95 < 150ms.
*   **Rotas**: `POST /run`, `GET /r/{cid}`, `POST /verify`.

### 9.2 AI Model Passport (P1)

*   **Acts**: ATTEST → EVALUATE.
*   **Policies**: existence + compliance/EU-AI + threshold de benchmarks.
*   **Proof**: **bundle**.
*   **Enrich**: badge, pdf, status.
*   **Billing**: `per_evaluation`.

### 9.3 Notarize-and-Prove (P1)

*   **Act**: ATTEST.
*   **Policies**: existence + provenance.
*   **Proof**: **bundle**.
*   **Enrich**: pdf, status.
*   **Billing**: `per_evaluation`.

_(Demais: Compliance Receipts, Metered Billing, Supply Chain, Proof-for-Access, SLA Enforcement — seguem o mesmo molde com trocas em `intake`/policies/enrich/proof.)_

* * *

10) Exemplo end-to-end (Gateway)
--------------------------------

1.  `POST /run` com:

```json
{
  "v":"ai-json-nrf1/0.1.1",
  "t":"record",
  "intent":{"kind":"EVAL","name":"api-receipt"},
  "ctx":{"method":"POST","path":"/v1/payments","body_cid":"b3:..."},
  "meta":{"tenant":"acme","app":"gw","user":"svc"},
  "decision":{}
}
```

2.  Produto → intake/event → policy/authorization → **ALLOW**.
3.  Host emite **permit**, grava WBE, executa enrich, **SIRP** INTENT/RESULT/DELIVERY/EXEC.
4.  Retorno:

```json
{ "cid":"b3:...", "verdict":"ACK", "card_url":"https://.../r/b3:..." }
```

5.  `GET /r/b3:...` (HTML/JSON) — mostra decisão e cadeia; baixar **bundle**.
6.  `POST /verify` com bundle — `OK`, PoI vazio.

* * *

11) Definition of Done — PRODUTO
--------------------------------

*    Manifesto resolvido e imutável (`cid` do product.json).
*    API exposta (`/run`, `/r/{cid}`, `/verify`, probes).
*    Pipeline implementado com módulos declarados (intake/policy/permit/sirp/verify/enrich).
*    **Permit** verificado pelo executor (nenhuma ação sem permit).
*    **SIRP** com cadeia íntegra; **bundle** verificável offline.
*    **Card URL** com negotiation (HTML/JSON); webhook assinado.
*    CI verde (unit/integration/property/fuzz/perf smoke).
*    Observabilidade e SLOs instrumentados.
*    Playbook de deploy/rollback (tags por `cid`).

* * *

12) Próximos passos práticos
----------------------------

1.  **Escolher P0**: **API Receipt Gateway** (confirmado).
2.  Gerar repositório `product-api-receipt-gateway` com scaffold:
    *   `/cmd/productd`, `/configs/product.json`, `/modules.lock`, `/charts/helm`.
3.  Integrar **intake-event@1**, **policy-authorization/gateway@1**, **permit-basic@1**, **sirp-standard@1**, **enrich-card\_url/webhook/ghost**, **verify-standard@1**.
4.  Subir **/run** + **/r/{cid}** + **/verify**; escrever 6 testes de integração (ACK/ASK/NACK, bundle, webhook, replay).
5.  Medir p95; ajustar limites; congelar 1.0.0.

se quiser, eu já te gero o **manifesto + rotas concretas** do Gateway e um **pacote de testes** (curl + pytest) pra rodar local e cravar o P0 agora. 💥💚

## Prompt:
Sim!

## Response:
Erro na análise

```python
