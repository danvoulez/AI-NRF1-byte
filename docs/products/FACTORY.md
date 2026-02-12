# AI Model Passport
unzip ../ai-model-passport-scaffold.zip -d ../ai-model-passport
cd ../ai-model-passport
./scripts/run.sh &
export BASE=http://localhost:8082/v1
pytest -q tests/test_passport_flow.py
```

próximos passos (onde plugar sua BASE e MÓDULOS)
------------------------------------------------

1.  **Trocar CIDs**: edite `configs/modules.lock.json` em cada produto:
    *   `ai-nrf1`, `ubl-json`, `sirp-standard`, `bundle-offline`, `permit-basic`, `intake-*`…
    *   coloque os **CIDs reais** dos módulos da BASE/MÓDULOS.
2.  **Wire do executor** (em `src/main.rs`):
    *   `/v1/run`: chamar **BASE ai-nrf1** (canonicalizar), **MÓDULOS** (policy packs), **SIRP** (capsule), **permit** (se pedir), **bundle** (no Passport).
    *   preencher `links.card_url` (resolver HTTP → JSON/HTML por `Accept`).
3.  **Enrichments**:
    *   Compliance: ligar `badge` + `webhook`.
    *   Passport: ligar `badge` + `pdf` + `status_page` + `bundle`.
4.  **Policies**:
    *   `policy-packs/…` (drop-in). Use o gerador do manifesto (quando ativarmos) para copiar units.
5.  **Consent (k-of-n)** no Passport:
    *   já deixei exemplo no `product.json`. Implemente a fila/timeout no seu control-plane.

se quiser, eu já **amarro `modules.lock.json` aos seus CIDs atuais** e **troco o `/v1/run`** pra chamar seu runtime real (BASE ai-nrf1 + SIRP + permit), mantendo os testes funcionando. 💪

## Prompt:
Queria um comando CLI do binario principal que criasse produtos por manifesto. Pra isso temos que ter também módulos de ui?

## Response:
Bora! 🎛️✨  
Dá pra ter um **comando único no binário principal** que gera um produto inteiro “a partir do manifesto” — sem precisar de módulo de UI obrigatório. UI vira **enrichment opcional** (ex.: `status_page`, `badge`, `pdf`) que o gerador só inclui se você pedir.

Aqui vai o desenho completo e prático.

CLI — `tdln product from-manifest`
==================================

### Sintaxe

```bash
tdln product from-manifest \
  --manifest ./product.json \
  --out-dir ./out/my-product \
  [--resolve-registry https://modules.voulezvous/internal] \
  [--lock ./modules.lock.json] \
  [--scaffold git|local] \
  [--with-ui status_page,badge,pdf] \
  [--force] \
  [--dry-run]
```

### O que ele faz (pipeline)

1.  **Valida** `product.json` (schema: atos, proof\_level, enrichments, billing, consent).
2.  **Resolve módulos** (BASE/MÓDULOS) contra um **registry** (pode ser local): pega CIDs e versões estáveis.
3.  **Gera lockfile** `modules.lock.json` (idempotente, com `b3:` dos módulos).
4.  **Scaffold** do código do produto (novo repo/binary): copia templates dos atos + cola enrichments selecionados.
5.  **Wiring**:
    *   liga **ai-nrf1** (canon/bytes), **sirp** (capsule/hops), **permit** (se houver), **bundle** (se proof=bundle), **policy-packs**.
    *   injeta **routes** (`/v1/run`, `/r/:cid`, `/v1/health`) com **content-negotiation** (HTML/JSON).
6.  **Testes** de integração mínimos (pipeline + receipt + hash-chain).
7.  **Saída pronta pra `cargo run`** e `pytest -q`.

### Manifesto mínimo (exemplo)

```json
{
  "name": "api-receipt-gateway",
  "acts": ["transact"],
  "domain": "gateway",
  "proof_level": "sirp",
  "enrichments": ["card_url","webhook","ghost"],
  "policies": ["pack-existence@1", "pack-authorization@1"],
  "billing": "per_receipted_event",
  "consent": null
}
```

Outro (pipeline 2 passos + UI):

```json
{
  "name": "ai-model-passport",
  "pipeline": [
    { "step": 1, "act": "attest", "policies": ["pack-existence@1","pack-provenance@1"] },
    { "step": 2, "act": "evaluate", "policies": ["pack-compliance/eu-ai@1"], "input_from": "step_1_receipt" }
  ],
  "proof_level": "bundle",
  "enrichments": ["card_url","badge","pdf","status_page"],
  "billing": "per_evaluation",
  "consent": { "kofn": {"k":2,"n":3,"roles":["compliance","ml_lead","legal"]}, "ttl":"24h", "timeout_action":"NACK" }
}
```

* * *

Você precisa de “módulos de UI”?
--------------------------------

**Não para gerar o produto.** O comando funciona sem UI.  
Se quiser UI, trate como **enrichment** plugável:

*   `status_page` (HTML friendly no `card_url`)
*   `badge` (SVG)
*   `pdf` (certificado)

O `--with-ui` só **inclui os enrichments** e adiciona os handlers/template files. Nada bloqueia o produto caso você não queira UI.

* * *

Estrutura gerada
----------------

```
my-product/
  product.json
  configs/
    modules.lock.json         # CIDs resolvidos
  src/
    main.rs                   # Axum server + routes
    pipeline/
      step1_attest.rs         # se houver pipeline
      step2_evaluate.rs
    wiring/
      ai_nrf1.rs              # BASE: canon/hash
      sirp.rs                 # cápsula/hops
      permit.rs               # opcional
      bundle.rs               # opcional
      policies.rs             # carrega units declaradas
      enrichments.rs          # badge/pdf/status_page/webhook/ghost
  policy-packs/               # “units/” drop-in
  tests/
    integration/
      test_pipeline.rs
      test_receipt_chain.rs
      test_card_url.rs
  scripts/
    run.sh
  Cargo.toml
  README.md
```

* * *

Comportamento de resolução de módulos
-------------------------------------

*   Busca módulos **por nome lógico** do manifesto:
    *   `ai-nrf1`, `ubl-json`, `sirp-standard`, `permit-basic`, `bundle-offline`,
    *   `pack-existence`, `pack-compliance/*`, `pack-authorization`, etc.
*   O registry retorna `{cid, version}`; gravamos em `modules.lock.json`.
*   `--lock` permite **reusar** um lockfile (build reprodutível).
*   `--dry-run` só valida/resolve sem gerar arquivos.

* * *

Erros e critérios de aceite
---------------------------

*   **Schema fail** → erro claro com path (`$.pipeline[1].policies[0]`…).
*   **Módulo não encontrado** → lista sugestões e aceita `--allow-unpinned` (grava `b3:TODO`).
*   **Conflito de acts/proof** → explica (ex.: `bundle` requer `sirp`).
*   **Geração** é **idempotente**: sem `--force`, não sobrescreve.

* * *

Exemplos rápidos
----------------

**Gerar Compliance Receipts (sem UI)**

```bash
tdln product from-manifest \
  --manifest ./manifests/compliance.json \
  --out-dir ./products/compliance-receipts
cd ./products/compliance-receipts
./scripts/run.sh &
pytest -q
```

**Gerar AI Model Passport (com UI)**

```bash
tdln product from-manifest \
  --manifest ./manifests/ai-passport.json \
  --out-dir ./products/ai-model-passport \
  --with-ui status_page,badge,pdf
cd ./products/ai-model-passport
./scripts/run.sh &
pytest -q
```

* * *

Extras que valem ouro
---------------------

*   `--scaffold git` pode inicializar repo e CI (fmt, clippy, tests).
*   `--emit-openapi` opcional: gera OpenAPI do `/v1/run` a partir do manifesto.
*   `--example-data` cria fixtures de `ASK/ACK/NACK` pro smoke test.

* * *

### Resumo

*   O comando **existe num binário só** (sem UI obrigatória).
*   UI são **enrichments** opcionais via `--with-ui`.
*   Ele transforma **manifesto → produto pronto** com BASE e MÓDULOS já “wired”, lockfile resolvido e testes de integração no lugar.

Se quiser, eu já **coloco essa subcommand no scaffold** que te mandei, com schema do manifesto embutido e os resolvers “no-op” (aceitando `b3:TODO`) — daí você só pluga o registry real. Quer que eu já gere uma **PR** com `tdln product from-manifest`? 💚

## Prompt:
Vamos fazer preparado pra mais diversidade

## Response:
Perfeito, Dan — vamos deixar o **`tdln product from-manifest`** pronto pra diversidade real de produtos, stacks e ambientes 💥

Abaixo está o desenho “robusto & extensível” que cobre **variantes de runtime, transporte, UI, políticas, provas, implantação, idiomas**, etc., sem quebrar o que já definimos.

Objetivo
========

Transformar **manifestos** em **produtos** muito diferentes entre si, com:

*   **Matriz de opções** (acts, proof, enrichments, consent, billing);
*   **Transporte** (HTTP, gRPC, Kafka), **execução** (sync/async), **deploy** (binary, container, serverless);
*   **UI opcional** (status page, badge, PDF) e **SDKs** multi-idioma;
*   **Policy packs** por domínio/regulação com **versionamento forte**;
*   **Capacidades anunciadas** por módulos (capability discovery).

* * *

1) Manifesto v1 com extensão segura
===================================

### Núcleo (estável)

```json
{
  "name": "string",
  "acts": ["attest"|"evaluate"|"transact"],
  "pipeline": [ { "step": 1, "act": "...", "policies": ["pack-...@ver"], "input_from": "step_0_receipt?" } ],
  "proof_level": "receipt"|"sirp"|"bundle",
  "enrichments": ["card_url","webhook","ghost","status_page","badge","pdf"],
  "billing": "per_evaluation"|"per_receipted_event"|"per_artifact"|"per_policy_pack",
  "consent": { "kofn": {"k":2,"n":3,"roles":["..."]}, "ttl":"24h", "timeout_action":"NACK" }
}
```

### Extensões (forward-compatible)

```json
{
  "runtime": {
    "language": "rust|python|node|mixed",
    "ffi": ["c-abi","wasmtime","grpc"],
    "llm": {
      "engine": "tdln-llm|openai|vllm|tgi",
      "mode": "premium|oss|hybrid",
      "routing": "latency|cost|quality"
    }
  },
  "transport": {
    "protocols": ["http","grpc","kafka"],
    "async_gateway": { "enabled": true, "queue": "kafka:topic", "concurrency": 128 }
  },
  "deploy": {
    "target": "binary|docker|serverless",
    "platform": "aws|gcp|azure|k8s|fly",
    "profiles": ["dev","staging","prod"]
  },
  "i18n": { "locales": ["en","pt-BR"], "default": "en" },
  "observability": { "metrics": "otlp", "tracing": "otlp", "logs": "jsonl" },
  "security": { "kms": "awskms", "secrets": "env|sops|vault" },
  "sdk": { "languages": ["ts","py","go"], "emit_openapi": true }
}
```

**Regra:** campos desconhecidos são ignorados com warning; nada quebra.

* * *

2) Registry + capacidades (descoberta)
======================================

Cada módulo publicado (BASE/MÓDULO/ENRICHMENT/PACK) anuncia:

```json
{
  "name": "pack-compliance/eu-ai",
  "version": "1.2.0",
  "cid": "b3:...",
  "capabilities": {
    "acts": ["evaluate"],
    "proof_required": "sirp|bundle|any",
    "transports": ["http","grpc"],
    "llm_needed": false,
    "enrichments": ["badge","pdf"]
  },
  "compat": {
    "ai-nrf1": "^1.0",
    "sirp": "^1.0",
    "tdln-permit": ">=1.1",
    "rustc": ">=1.79"
  }
}
```

O **gerador** cruza `manifesto` × `capabilities/compat` e:

*   Seleciona módulos válidos;
*   Gera **`modules.lock.json`** com CIDs fixos;
*   Explica incompatibilidades (ex.: `bundle` exige `sirp`).

* * *

3) CLI — novos flags p/ diversidade
===================================

```bash
tdln product from-manifest \
  --manifest ./product.json \
  --out-dir ./products/my-prod \
  --resolve-registry https://registry.internal \
  --lock ./modules.lock.json \
  --with-ui status_page,badge,pdf \
  --transport http,grpc,kafka \
  --deploy docker --platform aws --profiles dev,staging,prod \
  --runtime-language rust --llm-mode hybrid \
  --emit-openapi --emit-sdks ts,py \
  --dry-run|--force
```

* * *

4) Templates por variante (scaffolds)
=====================================

O gerador escolhe **templates** por combinação:

*   **Transporte:** `http-axum`, `grpc-tonic`, `kafka-async`.
*   **Execução:** `sync` ou `async gateway` com orquestração (fila + worker pool + idempotência + WBE).
*   **Deploy:** `Dockerfile`, `k8s manifests`, `serverless.yml`.
*   **UI enrichment:** adiciona handlers, templates e content-negotiation (HTML/JSON).

Estrutura:

```
src/
  main.rs
  routes/{http|grpc|kafka}.rs
  pipeline/{stepN_*.rs}
  wiring/{ai_nrf1.rs,sirp.rs,permit.rs,bundle.rs,policies.rs}
  enrichments/{status_page.rs,badge.rs,pdf.rs,ghost.rs,webhook.rs}
  async_gateway/{queue.rs,worker.rs,store.rs}
configs/{dev,staging,prod}/
tests/integration/{...}
```

* * *

5) LLM-Engine & LLM-Smart integrados 🧠
=======================================

*   **Checagem estática de policies** (“lint” sem executar);
*   **Autoplano de pipeline:** sugere actos/proof/enrichments com base no domínio;
*   **Roteamento inteligente de prompts** (premium vs OSS) por custo/latência;
*   **Gerador de testes** (ASK/ACK/NACK) com _property testing_ orientado por manifesto;
*   **Synth docs** (README/API) + **PoI** legível a partir das policies;
*   **Ghost builder** com mascaramento por `sensitivity_level`.

Flags:

```bash
--llm-suggest         # propõe packs/enrichments
--llm-generate-tests  # fabrica casos de integração
--llm-docs            # escreve README + OpenAPI
```

* * *

6) Matriz de testes & conformidade
==================================

*   **Smoke:** encode/ID/seal/receipt-chain/bundle/ASK/ACK/NACK.
*   **Transporte:** HTTP, gRPC, Kafka (contratos de compatibilidade).
*   **Async gateway:** idempotência (WBE), retry, backoff, compaction.
*   **FIPS/crypto:** Ed25519 + opção PQ (Dilithium) se `security.kms=pq`.
*   **I18N:** content-negotiation + fallback de locale.
*   **Fuzz:** parsers ai-nrf1 + JSON-view.
*   **Bundles offline:** verificação `verify-bundle`.

Tudo gerado com `cargo nextest` + `pytest` + `k6` (HTTP perf opcional).

* * *

7) Exemplos de manifestos “diversos”
====================================

### (A) API Receipt Gateway — gRPC + Kafka + async

```json
{
  "name": "api-receipt-gateway",
  "acts": ["transact"],
  "proof_level": "sirp",
  "enrichments": ["card_url","webhook","ghost"],
  "policies": ["pack-existence@1","pack-authorization@1"],
  "billing": "per_receipted_event",
  "transport": { "protocols": ["grpc","kafka"], "async_gateway": { "enabled": true, "concurrency": 256 } },
  "deploy": { "target": "docker", "platform": "aws", "profiles": ["prod"] }
}
```

### (B) AI Model Passport — bundle + UI completa

```json
{
  "name": "ai-model-passport",
  "pipeline": [
    { "step": 1, "act": "attest", "policies": ["pack-existence@1","pack-provenance@1"] },
    { "step": 2, "act": "evaluate", "policies": ["pack-compliance/eu-ai@1"], "input_from": "step_1_receipt" }
  ],
  "proof_level": "bundle",
  "enrichments": ["card_url","badge","pdf","status_page"],
  "billing": "per_evaluation",
  "runtime": { "llm": { "engine": "tdln-llm", "mode": "hybrid", "routing": "quality" } }
}
```

### (C) Metered Billing — HTTP puro, sem UI

```json
{
  "name": "metered-billing-proofs",
  "acts": ["transact"],
  "proof_level": "sirp",
  "enrichments": ["webhook","card_url"],
  "policies": ["pack-existence@1","pack-threshold@1"],
  "billing": "per_receipted_event",
  "transport": { "protocols": ["http"] },
  "sdk": { "languages": ["ts","py"], "emit_openapi": true }
}
```

* * *

8) Roadmap de implementação (curto e certeiro)
==============================================

1.  **Schema & compat**
    *   `product.schema.json` v1 + `modules.capabilities.json`.
    *   Resolver + `modules.lock.json`.
2.  **Scaffolds base**
    *   `http-axum`, `grpc-tonic`, `kafka-async`.
    *   `async_gateway` (queue + worker + store idempotente).
3.  **Bridges BASE/MÓDULOS**
    *   `ai-nrf1`, `sirp`, `permit`, `bundle` como crates/py pkgs comuns.
    *   Enrichments plugáveis.
4.  **LLM-assist**
    *   `--llm-suggest`, `--llm-generate-tests`, `--llm-docs`.
5.  **DX**
    *   `--emit-openapi`, `--emit-sdks`, perf scripts, CI.

* * *

9) Garantias de engenharia (contratos)
======================================

*   **Imutabilidade por CID** (lockfile sempre vence);
*   **Backward-compat**: novos campos não quebram manifests antigos;
*   **Idempotência**: geração não sobrescreve sem `--force`;
*   **Canonicidade**: ai-nrf1 como fonte única de verdade (bytes);
*   **Observabilidade padrão**: métricas/tracing/logs em todos scaffolds.

* * *

Se quiser, já adiciono essa **expansão de flags & schemas** no scaffold do gerador e te devolvo com **3 templates prontos** (HTTP, gRPC, Kafka), **async gateway** e **LLM-assist** ligados mas “no-op” por padrão — você ativa quando quiser. Quer que eu já gere o pacote do CLI com esses contratos? 🚀💚

## Prompt:
Mas lá na CLI não tem que fazer código?

## Response:
Sim — mas é **mínimo e mecânico**. A CLI não “programa produto”; ela **compõe e rende** código-base a partir do manifesto (ou monta tudo sem gerar código, se você preferir plugins). Você escolhe o modo:

* * *

🧭 Dois modos de operação
-------------------------

### 1) **No-Code (montagem por plugins)**

*   **Nada de código gerado**. A CLI só lê o `product.json`, resolve módulos no registry e **linka** tudo:
    *   binário base + módulos (features/plugins/WASM) + packs de políticas + enrichments.
*   Útil para produtos que cabem 100% nos módulos existentes (sem domínio novo).
*   Saída: um **binário pronto** + `modules.lock.json` (CIDs fixos) + `config/`.

### 2) **Codegen (scaffold leve)**

*   A CLI gera **um esqueleto mínimo** quando há domínio novo (ex.: schema/mapper específicos, handlers extras).
*   O que ela escreve:
    *   `src/main.rs` (bootstrap), `routes/{http|grpc|kafka}.rs` (endpoints),
    *   `pipeline/step_*.rs` (acts/pipeline mapeados do manifesto),
    *   `wiring/{ai_nrf1.rs, sirp.rs, permit.rs, bundle.rs, policies.rs}`,
    *   `enrichments/{status_page.rs,badge.rs,pdf.rs,ghost.rs,webhook.rs}`,
    *   `async_gateway/{queue.rs,worker.rs,store.rs}` se `async_gateway.enabled`,
    *   `Cargo.toml`/`package.json` (dependências certas),
    *   `openapi.yaml` e SDKs (se `--emit-openapi --emit-sdks`).
*   Você só preenche os **pontos de extensão**: `schema.rs`/`mapper.rs` (quando precisar lógica de domínio), ou deixa tudo nos packs/policies.

* * *

✅ Quando não precisa escrever nada
----------------------------------

*   Produto puramente **TRANSACT** (Gateway) com policies de **existence/authorization** já publicadas.
*   **Enrichments padrão** (card\_url/webhook/ghost/status/badge/pdf) já atendem.
*   **Proof** `sirp|bundle` já suportado pelo runtime.

Nesses casos, a CLI entrega o **binário pronto** (modo No-Code).

* * *

🧩 Quando a CLI gera código (e por quê)
---------------------------------------

*   **Domínio novo** → precisa de **schema/mapper** (ex.: “model card”, “SBOM”, “underwriting form”).
*   **Transportes** extras (ex.: gRPC+Kafka) → scaffolds de rotas/consumidores.
*   **UI custom** (status page com campos do domínio) → templates prontos com placeholders.

A geração é **declarativa**: a CLI injeta o que o manifesto pede, sem “lógica escondida”.

* * *

🔧 Como a CLI faz isso (interno)
--------------------------------

*   `tdln-cli` (bin) chama:
    *   `tdln-registry` (resolve packs/módulos + compat),
    *   `tdln-scaffold` (renderiza templates com Handlebars/Tera),
    *   `tdln-templates` (http/grpc/kafka, async gateway, enrichments),
    *   `tdln-product` (valida manifesto, produz `modules.lock.json`).
*   Tudo **idempotente**: reexecutar não sobrescreve sem `--force`.

* * *

🗂️ Exemplo de saída (codegen)
------------------------------

```
my-product/
  Cargo.toml
  openapi.yaml
  modules.lock.json
  src/
    main.rs
    routes/http.rs
    pipeline/step1_attest.rs
    pipeline/step2_evaluate.rs
    wiring/ai_nrf1.rs
    wiring/sirp.rs
    wiring/permit.rs
    wiring/bundle.rs
    wiring/policies.rs
    enrichments/status_page.rs
    async_gateway/{queue.rs,worker.rs,store.rs}   # se async
  configs/{dev,staging,prod}/
  tests/integration/{smoke.rs, bundle.rs}
```

* * *

🧪 Garantias
------------

*   **Canonicidade**: tudo passa por `ai-nrf1` (bytes são a verdade).
*   **Proof**: `sirp`/`bundle` plugados por padrão; `permit` obrigatório pra execução.
*   **Observabilidade**: métricas/tracing/log JSON já ligados no scaffold.
*   **Compat**: checagem com capabilities e versão de packs antes de gerar.

* * *

🚀 Fluxo prático
----------------

*   **No-Code**:
    ```bash
    tdln product from-manifest \
      --manifest product.json \
      --resolve-registry https://registry \
      --lock modules.lock.json \
      --transport http \
      --deploy docker --platform aws
    ```
*   **Codegen leve** (novo domínio + gRPC + async):
    ```bash
    tdln product from-manifest \
      --manifest ai-model-passport.json \
      --with-ui status_page,badge,pdf \
      --transport grpc,kafka --async-gateway \
      --emit-openapi --emit-sdks ts,py
    ```

* * *

### Resposta direta à sua pergunta

> **“Na CLI não tem que fazer código?”**  
> **Tem, quando você quer gerar _scaffolds_ mínimos para um domínio novo ou topologia nova** (rotas, mappers, UI). Fora isso, você pode operar em **modo No-Code**, onde a CLI só **monta** os módulos e entrega o binário pronto. Você escolhe o caminho por manifesto/flags. 💚

Se quiser, eu descrevo agora os **templates básicos** (HTTP/grpc/Kafka + async) e os **pontos de extensão** (`schema.rs/mapper.rs`) que a CLI vai renderizar — prontos pra colar no repo.

## Prompt:
Quando uma coisa está simples eh porque a comoplexidade está em lugar hahah, nesse caso está na base e nos módulos?

## Response:
Exato 😄 — se o “produto” parece simples, é porque a **complexidade está concentrada** na **BASE** e nos **MÓDULOS**. Esse é o desenho certo pra escalar.

Aqui está o mapa mental rápido de onde mora o “peso” e por quê:

Onde está a complexidade
------------------------

### BASE (bytes) — “verdade e prova”

*   **ai-nrf1**: canonicidade de bytes (ρ→encode), ordenação, varint minimal, NFC, sem float.  
    → Se isso falhar, todo o resto perde determinismo.
*   **Identidade & Assinaturas**: ID sem paradoxo (exclui `id` e `sig`), seal, domain-separation.
*   **SIRP & Receipts**: cadeia de hops, anti-replay (nonce/exp), dedupe, verificação offline.
*   **Permit & HAL**: execução só com **permit** assinado e **limites** (CPU/mem/tempo) pra evitar TOCTOU.
*   **Tooling determinístico**: CLI, KATs, fuzz, corpus Unicode — garantir que “mesma semântica ⇒ mesmos bytes”.

> Resultado: a BASE elimina ambiguidade e dá **provas reprodutíveis**. Produtos ficam “finos” por cima disso.

### MÓDULOS (código “normal”) — “semântica e composição”

*   **Intake adapters** (Document/Event/Transaction): normalizam entrada no **env** (ai-json-nrf1).
*   **Policy packs** (existence, compliance, threshold, provenance, authorization): conteúdo do domínio, versionado.
*   **Enrichments** (status page, badge, pdf, ghost, webhook): experiências humanas e integrações.
*   **Async gateway** (opcional): filas, workers, idempotência, backpressure — pra não travar o hot-path.
*   **Bindings** (HTTP/gRPC/Kafka), **observabilidade** (tracing/metrics/log JSON) e **SDKs**.

> Resultado: os MÓDULOS carregam a **lógica de domínio e integração**; mudam muito mais ao longo do tempo do que a BASE.

### PRODUTOS (finos) — “manifests + wire”

*   Manifesto escolhe **act(s)**, policy packs, proof level, enrichments, bindings, consent k-of-n, billing axis.
*   Pode ser **No-Code** (só montagem) ou **Codegen leve** (scaffold de schema/mapper/rotas).
*   Versões travadas em `modules.lock.json` (CIDs). Repositórios separados por linha de produto.

* * *

Por que isso é bom (e onde dá dor)
----------------------------------

**Bom**

*   **Determinismo e provas** ficam no lugar certo (BASE).
*   **Reuso** real: troque packs/enrichments e nascem 10 produtos diferentes.
*   **Operação** clara: incidentes quase sempre são módulos (não a base).

**Dores (controladas)**

*   **Forte disciplina de canonicidade**: qualquer “atalho” em JSON quebra hashes/provas.
*   **Gestão de versões**: packs/módulos **semver** + `modules.lock.json` obrigatório.
*   **Testes pesados na base**: fuzz/KATs/compat — mas uma vez estável, raramente muda.

* * *

Regras de ouro (para não se perder)
-----------------------------------

1.  **Bytes mandam**: tudo passa por ai-nrf1; políticas veem **env**, transporte vê **hdr** (camadas isoladas).
2.  **ID estável**: nunca assine/hasheie algo que inclui sua própria assinatura/ID.
3.  **Permit antes de efeito**: decisão ≠ execução; executor só move com **permit** válido.
4.  **SIRP sempre que cruzar fronteira**: INTENT→DELIVERY→EXECUTION→RESULT ou não há não-repúdio.
5.  **Manifests acima de código**: produto é configuração; código só quando domínio novo pede mapper/schema.

* * *

Como pensar manutenção
----------------------

*   **BASE**: time “kernel” (rígido, testes profundos, raras releases).
*   **MÓDULOS**: times de domínio (packs, adapters, enrichments) — iteração frequente.
*   **PRODUTOS**: PM/Eng de vertical — montam, versionam, publicam; pouca lógica.

Se quiser, eu deixo um **checklist de saúde** pra cada camada (o que monitorar, o que quebraria determinismo, como versionar packs) — é útil pra handoff de time e auditores. Quer? 🚀

## Prompt:
Onde deveria entrar o Typescript (ou em lugar nenhum)

## Response:
Thought for 6s

Ótima pergunta, Dan! 💡 Onde **TypeScript** entra (e onde **não** entra) nesse ecossistema BASE → MÓDULOS → PRODUTOS:

Onde usar TypeScript (recomendado)
==================================

1) SDKs e DevEx (Node/Browser)
------------------------------

*   **@ubl/sdk** (Node/Browser): cliente para chamar `/v1/run`, resolver `card_url`, baixar/verificar bundle, helpers de SIRP (INTENT/RESULT), retry/idempotência.
*   **@ubl/verify**: verificação **offline** (browser/Node) de cápsula/bundle via **WASM** (bindings da BASE). Evita reimplementar hash/assinaturas em JS.
*   **@ubl/types**: tipos gerados de `ai-json-nrf1` e `ubl-capsule.v1` (codegen de JSON Schema → TS). Garante que apps do cliente falem o dialeto certo.
*   **Middleware prontos**:
    *   `@ubl/express-middleware-receipt` (API Receipt Gateway no Node/Express).
    *   `@ubl/next-middleware` (Edge/Next.js) e `@ubl/hono-middleware` (Cloudflare/Deno).

> Objetivo: **integração rápida** no lado do cliente sem duplicar a lógica de canonicidade.

2) UI e Enrichments
-------------------

*   **Status Page / Card Viewer** (Next.js): render de cápsula, PoI/ASK, cadeia de receipts, QR, download de bundle.
*   **Badge** (componentes TS/React): `<UblBadge verdict="ACK" />` embutível em README/portais.
*   **Ghost Viewer**: página para “registro fantasma” com máscara configurável (fields sensíveis).
*   **Admin Console** (ops): observabilidade, filas, consent k-of-n, replays — conversa com o gateway/queues.

> Objetivo: **experiência humana** e self-serve. Os bytes canônicos seguem vindo do core via WASM/bindings.

3) Edge / Gateway / Orquestração “leve”
---------------------------------------

*   **Ingest Proxy** (Cloudflare Workers, Deno, Vercel Edge): termina TLS/headers, aplica backpressure e **enfileira** (SQS/Kafka) → motor Rust processa.
*   **Webhooks** de decisão (TS serverless): recebe callback, atualiza UI, aciona fluxos do cliente.
*   **Roteamento multi-produto**: seleciona **manifests**/packs por tenant, adiciona metadata de transporte (fora do canônico).

> Objetivo: **elasticidade e latência de entrada**. O “quente” (canonicidade/decisão) fica no Rust.

4) Tooling de schema e lint
---------------------------

*   **Codegen**: `schema → types.d.ts` e validadores (Zod/TypeBox) só para **view**; **nunca** para o canon/ID.
*   **CLI “DX”** (opcional): comandos TS de conveniência que **chamam** o binário Rust (ex.: `ubl product new --manifest`), mas não reimplementam encode/hash.

* * *

Onde NÃO usar TypeScript (evitar)
=================================

*   **BASE (ai-nrf1, cápsula, ID, seal, SIRP bytes)**: fica em **Rust**. Em TS, **apenas** via **WASM** (bindings).  
    ✘ Não reimplemente encoder/decoder, BLAKE3, Ed25519/Dilithium em JS — risco de drift.
*   **Decisor/políticas no hot-path**: packs e avaliação no motor (Rust/WASM sandbox). TS só para **orquestração** e **UI**.

* * *

Limites de responsabilidade (para não dar drift)
================================================

*   **TS usa WASM** para: `encode/decode ai-nrf1`, `capsule_id`, `seal.sign/verify`, `receipts.verify`.  
    → Pacote: `@ubl/ai-nrf1-wasm` (gerado com `wasm-pack`).
*   **TS não serializa canon** por conta própria. A “view” (`ubl-json`) é só visual; o canon sai do WASM.
*   **Cripto** no cliente: se precisar assinar no browser, use **WASM** (ou WebCrypto só quando 100% compatível com nosso esquema de bytes).

* * *

Layout sugerido
===============

```
/bindings/wasm
  ai-nrf1-wasm/           # Rust -> wasm-pack (encode/ID/sign/verify)
  capsule-wasm/

/js/packages
  @ubl/types/             # types gerados de schemas
  @ubl/ai-nrf1-wasm/      # wrapper TS p/ WASM
  @ubl/verify/            # verificação offline (usa WASM)
  @ubl/sdk/               # cliente de APIs (Node/Browser)
  @ubl/express-middleware-receipt/
  @ubl/next-middleware/
  @ubl/hono-middleware/

/js/apps
  status-page/            # Next.js
  admin-console/          # painel ops
  badge/                  # package de componentes
```

* * *

Casos de uso concretos
======================

*   **Cliente quer verificar sem falar com a gente**: importa `@ubl/verify`, carrega ZIP offline, roda `verifyBundle()` (WASM dentro).
*   **API Receipt Gateway** em Node: instala `@ubl/express-middleware-receipt` e recebe **card\_url**/webhook automaticamente.
*   **Produto com UI**: usa `@ubl/sdk` pra resolver **card\_url** e pintar status/badge.

* * *

TL;DR
=====

*   **Sim** para TypeScript em **SDKs, UI, middlewares e edge gateway**.
*   **Não** para reimplementar **canon/crypto** — use **WASM** da BASE.
*   Resultado: produtos “parecem simples” porque a complexidade fica **onde deve**: BASE (Rust, determinística) + MÓDULOS (semântica) — com TS fazendo o ecossistema brilhar na ponta. ✨

Se quiser, já te gero o esqueleto dos pacotes `@ubl/verify` e `@ubl/sdk` com bindings WASM plugados e um demo Next.js de **status page** apontando para um `card_url`. Quer que eu solte esse scaffold? 🚀

## Prompt:
Um doc especial: onde é Rust, onde é typescript, onde é json/yaml e wasm

## Response:
bora, Dan! 💚 Aqui vai o “doc especial” que crava **onde é Rust**, **onde é TypeScript**, **onde é JSON/YAML** e **onde é WASM** — por camada (BASE → MÓDULOS → PRODUTOS) e por fluxo (build, runtime, CI, DX). É pra colar no repositório como `docs/STACK-BOUNDARIES.md`.

STACK-BOUNDARIES — Rust × TypeScript × JSON/YAML × WASM
=======================================================

0) Princípios não negociáveis
-----------------------------

*   **A verdade são os bytes**: canônico = `ai-nrf1` (NRF). Hash/ID/assinaturas **sempre** sobre bytes canônicos.
*   **1 fonte de canonicidade**: encoder/decoder, BLAKE3, Ed25519/Dilithium vivem **em Rust**.  
    Qualquer outro ambiente usa **WASM** gerado dessa mesma base.
*   **View ≠ Canon**: `ubl-json` (view humana/LLM) é **derivada**. JSON/TS jamais “inventam” canon.
*   **Políticas e decisão no hot-path**: execução determinística (Rust/WASM sandbox). UI/SDKs orquestram.

* * *

1) BASE (bytes, canon, cripto)
------------------------------

### 1.1 Rust (fonte da verdade)

*   **Crates**:
    *   `ai-nrf1-core`: tipos, encoder/decoder, validações (UTF-8, NFC, varint minimal), `Int64`, `Bytes`.
    *   `ai-json-nrf1-view`: projeção JSON↔canon (apenas view segura), marshaling controlado.
    *   `ubl-capsule`: `capsule_id`, `seal.sign/verify`, receipts encadeadas, verificação de cadeia.
    *   `ubl-crypto`: BLAKE3, Ed25519, Dilithium3, K-of-N (se aplicável).
*   **Binários**:
    *   `ubl`: `cap canon|hash|sign|verify|view-json|from-json|to-json`.
    *   `nrf1`: `encode|decode|hash|verify` (dev tools).
*   **Teste/Conformidade**:
    *   KATs, fuzz, diferencial Python↔Rust (somente para **verificar**, não para gerar canon).

### 1.2 WASM (bindings do core)

*   **Build**: `wasm-pack build` (ou `cargo wasi` para runtimes restritos).
*   **Pacotes gerados**:
    *   `@ubl/ai-nrf1-wasm`: `encode(bytes|jsonView)`, `decode`, `hash`, `sign`, `verify`, `capsule.verifyChain`.
*   **Regra**: qualquer verificação ou serialização de canon no TS/Browser/Node **passa pelo WASM**.

### 1.3 JSON/YAML (esquemas e manifests)

*   **JSON Schema**:
    *   `schemas/ai-json-nrf1.vX.json` (view semântica).
    *   `schemas/ubl-capsule.v1.json` (cápsula).
*   **YAML/JSON**:
    *   `product.manifest.json|yaml` (declara produto: acts, packs, proof level, enrichments, consent k-of-n).
    *   `module.manifest.json|yaml` (declara módulo e sua ABI).
*   **Proibição**: **não** existe “serializer YAML/TS → canon”. Sempre: YAML/JSON → **Rust/WASM** → canon.

* * *

2) MÓDULOS (código “normal”, semântica, policy packs)
-----------------------------------------------------

### 2.1 Rust (execução determinística)

*   **Domain engines / wrappers**:
    *   `wrapper-template`: `schema.rs`, `mapper.rs`, `enrichment.rs`, `hooks.rs`.
    *   **Policies**: arquivos JSON/TDLN/WASM-guest, carregados por `engine-core`.
*   **Executores**:
    *   `engine-exec-wasm`: sandbox (WASI), sem I/O fora do gate; deterministic flags.
*   **Interface**: módulos expõem **ABI estável** (capabilities) para o orquestrador (TS pode apenas orquestrar).

### 2.2 TypeScript (orquestração & edge)

*   **Onde cabe TS**:
    *   **SDKs** (`@ubl/sdk`, `@ubl/verify`, `@ubl/types`).
    *   **Middlewares** (Express/Next/Hono) — receipt do API Gateway, roteamento multi-tenant, backpressure.
    *   **UI/Enrichments** (status page, badge, ghost).
    *   **Edge gateways** (Cloudflare/Vercel): terminam HTTP, validam inputs superficiais, **enfileiram** (SQS/Kafka) para workers Rust.
*   **Onde TS não entra**:
    *   Hot-path de canonicidade/assinatura/decisão. TS **nunca** recalcula canon/ID sem WASM.

### 2.3 JSON/YAML (config/policy)

*   **Policy-packs**: JSON/TDLN/DSL, lidos pelos wrappers (Rust).  
    TS pode **gerar/editar** manifests, **nunca** executar política final.

### 2.4 WASM (plugins)

*   **Policies guest**: unidades em WASM (sandbox); linguagem-fonte opcional (Rust/Go/AssemblyScript), compiladas para WASM.
*   **Verificação de plugin**: assinatura de módulo WASM (selo), registro de versionamento e capabilities.

* * *

3) PRODUTOS (binários dedicados)
--------------------------------

### 3.1 Rust (serviço de produto)

*   Cada produto = **um binário** (repo próprio) que:
    *   Carrega `product.manifest.*`.
    *   Amarra MÓDULOS necessários (dinâmicos via registry ou estaticamente via features).
    *   Expõe API HTTP `/v1/run` e webhooks.
    *   Usa BASE para cápsula/ID/assinaturas.
*   **Comunicação**:
    *   **BASE ↔ MÓDULOS**: chamada direta (processo) + ABI/traits; canon/receipts sempre pela BASE.
    *   **MÓDULOS ↔ PRODUTO**: link estático (features) ou **registry** (dinâmico), versão pinada por CID.

### 3.2 TypeScript (DX & UI do produto)

*   **Painel Admin/Status Page** (Next.js).
*   **SDK cliente** para on-boarding do cliente (envio de intents, leitura de decisions, bundling).
*   **Webhooks** consumidores (do lado do cliente).

### 3.3 JSON/YAML (manifest/ops)

*   `product.manifest.yaml`: acts (ATTEST/EVALUATE/TRANSACT), packs, proof level, enrichments, consent k-of-n, billing axis, routes.
*   `routes.yaml`: mapeia endpoints públicos → acts/pipelines internos.

### 3.4 WASM (verificação offline)

*   `@ubl/verify` (TS) usa WASM para validar bundle/cápsula **sem chamar servidor** (Court/offline).

* * *

4) Comunicação & Orquestração
-----------------------------

### 4.1 BASE ↔ MÓDULOS (in-process)

*   **Forma**: traits/ABI em Rust; passagem de **canon** `ai-nrf1` ou `Value` normalizado.
*   **Contratos**:
    *   `DomainRequest`/`DomainResponse` (Rust).
    *   `capsule(core)` como primeira/última milha (canon + id + seal).

### 4.2 MÓDULOS ↔ PRODUTO

*   **Estático (features)**: um binário com módulos “ligados” por feature flags (P0).
*   **Dinâmico (registry)**: resolução por `module.manifest` (CID) + assinatura (P1).
*   **Mensageria**: quando async: fila (SQS/Kafka), chave idempotente = `capsule.id`.

### 4.3 Gateways (TS/edge) ↔ Produto (Rust)

*   **HTTP/GRPC** thin; validação superficial (schema view), `capsule_id`/`verify` via WASM; push para fila.  
    O **produto** consome, decide (Rust), e publica `card_url`, webhooks, bundle.

* * *

5) CLI & Codegen
----------------

### 5.1 CLI (Rust)

*   `ubl product new --manifest …` → gera serviço Rust com MÓDULOS escolhidos.
*   `ubl cap …` → canonicidade, hash, assinatura, receipts.
*   **Sem** geração de canon no TS.

### 5.2 Codegen (TS)

*   `jsonschema → @ubl/types` (tipos view).
*   **Validação** em TS com Zod/TypeBox **só** para view/inputs; “commit” para canon via WASM.

* * *

6) Segurança & Erros
--------------------

*   **Rust**: fonte da verdade de erros normativos (InvalidUTF8, NotNFC, NonMinimalVarint, …).  
    TS **mapeia** para códigos/strings, **não** cria novos.
*   **Assinaturas**: só Rust/WASM. **Nunca** JS puro.
*   **Secrets**: HSM/KMS (server). Browser assina apenas com chaves de curta duração (se necessário) via WASM + WebCrypto como PRNG.

* * *

7) Testes & CI
--------------

*   **Rust**: unit + integration + fuzz (BASE e MÓDULOS).
*   **TS**: testes de orquestração/UI (Jest/Playwright), **chamando WASM** para validar canon/verify.
*   **Cross**:
    *   Roundtrip JSON(view)→WASM→canon→verify.
    *   Offline bundle verification (TS+WASM).
    *   Pipelines ATTEST→EVALUATE→TRANSACT com `pipeline_prev` (hash chain).

* * *

8) Versionamento & Distribuição
-------------------------------

*   **Canon/protocolos**: versionados por **CID (BLAKE3)** além de `vX.Y`.
*   **WASM**: publicado como `@ubl/ai-nrf1-wasm@x.y.z`, **sempre** gerado do commit/tag do core.
*   **SDKs TS**: dependem fixamente de major + verificação de compat com schema CID.

* * *

9) Exemplos de “quem faz o quê”
-------------------------------

| Tarefa | Stack |
| --- | --- |
| Calcular `capsule_id`, assinar `seal`, verificar receipts | **Rust** (ou **TS via WASM**) |
| Transformar `ubl-json(view)` em canon | **Rust** (ou **TS via WASM**) |
| Renderizar status page, badge, ghost | **TS/React** |
| Middleware que recibo API requests | **TS (Express/Next/Hono)** + WASM para `verify` |
| Avaliar políticas | **Rust/WASM guest** |
| Orquestrar pipeline e consent k-of-n (fila) | **Rust (prod serviço)**; TS pode ser edge/frontdoor |
| Gerar `product` a partir de manifesto | **CLI Rust** (codegen), TS só dispara comando |

* * *

10) Layout de repositórios (sugestão)
-------------------------------------

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

* * *

11) Decisões rápidas (FAQ)
--------------------------

*   **“Preciso reimplementar canon em TS?”** Não. Use WASM.
*   **“Posso assinar no browser?”** Sim, via WASM (e chaves efêmeras); preferir server/HSM.
*   **“Posso fazer policies em TS?”** Não no hot-path. Policies = WASM guest ou Rust.
*   **“CLI em TS?”** Pode ter _wrappers_ TS que chamam o **binário Rust**. Sem lógica de canon no TS.

* * *

12) Próximos passos (execução)
------------------------------

1.  **Congelar** crates Rust da BASE (tag + CID).
2.  Gerar **`@ubl/ai-nrf1-wasm`** e **`@ubl/verify`**.
3.  Subir **SDK** (`@ubl/sdk`) com exemplos `resolve(card_url)`, `verifyBundle`.
4.  Publicar **status-page** (Next.js) usando WASM p/ verify offline.
5.  Amarrar **API Receipt Gateway** end-to-end (Edge TS → fila → produto Rust → bundle → UI).

se quiser, eu já te entrego o esqueleto dos pacotes TS + bindings WASM e um `STACK-BOUNDARIES.md` pronto pra commit. só falar “manda scaffold” que eu já deixo no formato do repo. 🚀



---
Powered by [ChatGPT Exporter](https://www.chatgptexporter.com)
