# Guia do Gerador de Produtos

> "A BASE e a verdade. Os MODULOS sao as ferramentas. O PRODUTO e so um JSON."

---

Parte 1 — Visão e Contrato
==========================

O que é o Gerador
-----------------

**`ubl product from-manifest`** transforma um **manifesto de produto** (`product.json`) + um **lock** (`modules.lock.json`) em um **binário autônomo** (ou container) que:

*   expõe `POST /evaluate` (recebe JSON, roda pipeline, emite recibo com **CID**),
*   assina/encadeia recibos e devolve **`url_rica`**,
*   roda **sem código específico de produto** (no-code/low-code, só manifestos).

**North Star:** “A BASE é a verdade. Os MÓDULOS são as ferramentas. O PRODUTO é só um JSON.”

* * *

Escopo do MVP (o que entra)
---------------------------

*   **Entrada:** `product.json` + (opcional) `modules.lock.json`.
*   **Saída:** binário com rotas `POST /evaluate`, `GET /health`, `GET /version`.
*   **Execução:** usa `module-runner` real (CapRegistry com 8 módulos).
*   **Flags:** `--out`, `--lock`, `--vars`, `--dry-run`, `--explain`, `--container`.
*   **Lock por CID:** pin de módulos por **BLAKE3** (quando fornecido).
*   **Sem state externo obrigatório** (resolver/registry pode ser outro serviço).

Fora do MVP (fica para v2):

*   Pull automático de módulos via registry remoto (pelo CID).
*   Assinatura do `modules.lock.json`.
*   Multi-tenancy/quotas/limiter avançado.
*   Cache-by-CID do LLM (planejado).

* * *

Contrato de Arquivos
--------------------

### 1) `product.json` (manifesto de produto)

**Shape compatível com o runner** (objetivo: zero cola custom).

**Campos (mínimo viável):**

*   `v`: número do schema do manifesto (ex.: `1`).
*   `name`: nome lógico do produto (ex.: `"tdln-policy"`).
*   `version`: string livre do produto (ex.: `"1.0.0"`).
*   `pipeline`: lista ordenada de **steps**:
    *   `step_id`: id único no pipeline (ex.: `"step-1-cap-policy"`).
    *   `kind`: **capability** (ex.: `"cap-intake"`, `"cap-policy"`, `"cap-runtime"`, `"cap-llm"`, `"cap-enrich"`, `"cap-transport"`).
    *   `version`: **major** ou `"*"` (o runner-real casa por major; `"*"` aceita qualquer).
    *   `config`: objeto com parâmetros do módulo.
*   `outputs` (opcional): preferências de saída, p.ex. `{ "format": "json" }`.

**Exemplo enxuto (policy):**

```json
{
  "v": 1,
  "name": "tdln-policy",
  "version": "1.0",
  "pipeline": [
    {
      "step_id": "step-0-cap-intake",
      "kind": "cap-intake",
      "version": "0",
      "config": { "map": [{ "from": "data", "to": "payload" }] }
    },
    {
      "step_id": "step-1-cap-policy",
      "kind": "cap-policy",
      "version": "0",
      "config": {
        "rules": [
          { "EXIST": ["payload"] }
        ]
      }
    },
    {
      "step_id": "step-2-cap-enrich",
      "kind": "cap-enrich",
      "version": "0",
      "config": { "providers": ["status_page"] }
    },
    {
      "step_id": "step-3-cap-transport",
      "kind": "cap-transport",
      "version": "0",
      "config": { "node": "did:ubl:local", "relay": [] }
    }
  ],
  "outputs": { "format": "json" }
}
```

> **Aliases opcionais** (se quiser no authoring): você pode escrever `policy.light`, `cap-llm-engine`, `cap-llm-smart`. O Gerador resolve para `kind` reais antes de validar/rodar.

* * *

### 2) `modules.lock.json` (pin matemático)

Trava **cada capability por CID** dos bytes canônicos. Garante reprodutibilidade.

**Campos:**

*   `spec`: versão do schema do lock (ex.: `1`).
*   `locked_at`: timestamp RFC3339.
*   `modules`: mapa `"kind@major" → { cid, bytes }"`.

**Exemplo:**

```json
{
  "spec": 1,
  "locked_at": "2026-02-12T11:22:33Z",
  "modules": {
    "cap-intake@0":    { "cid": "b3:...", "bytes": "path:modules/cap-intake/target/release/libcap_intake.rlib" },
    "cap-policy@0":    { "cid": "b3:...", "bytes": "path:modules/cap-policy/target/release/libcap_policy.rlib" },
    "cap-enrich@0":    { "cid": "b3:..." },
    "cap-transport@0": { "cid": "b3:..." }
  }
}
```

**Regras:**

*   Se houver `lock`, o Gerador valida que `kind@major` do `product.json` bate com o **CID** do lock (falha se divergir).
*   `bytes` pode ser `path:` (local) ou omitido quando o módulo já está embutido no build.

* * *

Contrato HTTP do Binário Gerado
-------------------------------

### `POST /evaluate`

**Request:** JSON livre (vira `env` via json\_view; `--vars` também entram no `env`).

```json
{ "data": "hello", "tenant": "acme" }
```

**Response (200):**

```json
{
  "product": "tdln-policy",
  "verdict": "Allow",
  "receipt_cid": "b3:...",
  "url_rica": "https://resolver.local/r/b3:...",
  "artifacts": [
    { "kind": "status_page", "cid": "b3:..." }
  ],
  "metrics": [
    { "step": "step-1-cap-policy", "metric": "duration_ms", "value": 0 }
  ],
  "stopped_at": null
}
```

**Erros:**

*   `400` — schema inválido / config inválida / input inválido.
*   `422` — pipeline inválido (ex.: kind não registrado).
*   `500` — erro interno (com `error_id` para correlação).

### `GET /health` → `200 OK`

### `GET /version` → `{ "version": "...", "git_sha": "...", "build_ts": "..." }`

* * *

Matriz de Execução / Compatibilidade
------------------------------------

| Modo | Descrição | Requisitos | Uso típico |
| --- | --- | --- | --- |
| **Stub** | compila rápido, sem IO externo | sem providers | dev local rápido |
| **Runner-real** | executa pipeline real com 8 módulos | features reais habilitadas | smoke/E2E |
| **Container** | OCI com bin gerado | Docker/Podman | deploy |

> O bin é **self-contained**. Resolver/Registry (para `url_rica`) pode ser um serviço separado (recomendado).

* * *

Critérios de Aceite (Parte 1)
-----------------------------

1.  Qualquer pessoa do time consegue escrever um `product.json` **válido** usando apenas esta seção.
2.  O que é “obrigatório” e “opcional” está claro (para manifesto, lock e HTTP).
3.  O contrato de resposta (`receipt_cid`, `url_rica`, `metrics`, `artifacts`) está inequívoco.

* * *


Parte 2 — Arquitetura Técnica do Gerador
========================================

Objetivo
--------

Transformar **`product.json`** (+ opcional **`modules.lock.json`**) em um **binário autônomo** (ou container) que expõe `/evaluate`, validando/rodando o pipeline real via `module-runner` com **CapRegistry** preenchido e **módulos pinados por CID** quando houver lock.

* * *

Pipeline interno (passo-a-passo)
--------------------------------

1.  **Carregar insumos**
    *   Ler `product.json` (schema `v=1`).
    *   _Opcional_: ler `modules.lock.json`.
    *   _Opcional_: `--vars k=v` (defaults de ambiente do produto).
    *   _Opcional_: `--explain` (gera visualização expandida do pipeline).
2.  **Resolver aliases & normalizar**
    *   Mapear aliases autorais → kinds reais:
        *   `policy.light` → `cap-policy`
        *   `cap-llm-engine` / `cap-llm-smart` → `cap-llm` (com `provider_hint` no `config`)
        *   `cap-structure` → `cap-intake`
    *   Garantir `version` por step (major numérico ou `"*"`).
3.  **Validar schema**
    *   Campos obrigatórios: `v`, `name`, `version`, `pipeline[*].{step_id,kind,version,config}`.
    *   Unicidade de `step_id`.
    *   Tipos básicos dos `config` por `kind` (validadores leves).
4.  **Reconciliar LOCK (se presente)**
    *   Para cada `kind@major`, verificar entrada correspondente em `modules.lock`.
    *   Checar **CID** do artefato (via BLAKE3 do bundle/rlib/wasm).
    *   Se divergente → **falha** (determinismo acima de tudo).
    *   Se ausente mas exigido por política de build → **falha** (ou `--allow-unlocked`).
5.  **Descobrir bytes dos módulos**
    *   Estratégias (na ordem):
        1.  **Embeds internos** (módulos já linkados no build runner-real — zero IO).
        2.  **Bytes locais** (paths no lock: `path:...`).
        3.  _(v2)_ **Registry remoto** por CID (pull).
    *   Resultado: **CapRegistry** com construtores das capabilities (não precisa carregar bytes em memória se estão linkadas — só registrá-las).
6.  **Montar CapRegistry**
    *   Registrar 8 capabilities (intake, policy, enrich, transport, permit, pricing, runtime, llm).
    *   Associar **major**/matcher de versão:
        *   `"*"` → aceita qualquer.
        *   `"0"` → checa que `api_version.major == 0`.
    *   Se lock existir, colar metadados de CID para telemetria/headers de recibo.
7.  **Gerar servidor (binário)**
    *   Injetar **manifesto resolvido** numa estrutura imutável (embed como JSON).
    *   Iniciar servidor HTTP minimalista:
        *   `POST /evaluate` → `json_view → runner.run(manifest, env) → resposta`.
        *   `GET /health` → 200.
        *   `GET /version` → `{ version, git_sha, build_ts }`.
    *   **Observabilidade**: logs estruturados por request (cid/hop/request\_id).
8.  **(Opcional) Empacotar container**
    *   `--container` gera Dockerfile multi-stage ou constrói OCI local.
    *   User não-root, `HEALTHCHECK`, `PORT` via env.

* * *

Decisões de engenharia (com recomendação)
-----------------------------------------

### A) **Embed vs. Dependência externa**

*   **Embed (recomendado no MVP)**  
    Prós: build único, simples, zero IO.  
    Contras: aumenta tamanho do bin; troca de módulo requer rebuild.
*   **Bytes externos via `modules.lock`**  
    Prós: reprodutibilidade por CID, swap controlado.  
    Contras: precisa resolver carregamento (rlib/wasm) e compat do ABI.

> **Recomendação**: MVP **embed** + lock apenas como **verificação semântica** (CID referencial). v2 adiciona “pull”/carregamento por CID.

### B) **Formato do módulo**

*   **Rust nativo (linkado, trait impl) — hoje**  
    Prós: performance, zero FFI.  
    Contras: precisa rebuild para atualizar.
*   **WASM “Certified Runtime” — progressivo**  
    Prós: sandbox, hot-swap, auditoria de bytes.  
    Contras: precisa API estável, host funcs e pre/post json\_view.

> **Recomendação**: manter **nativo** para as 8 caps existentes; evoluir **cap-runtime** para executar códigos do cliente em WASM (já previsto).

### C) **Versionamento**

*   `product.json` usa **major** ou `"*"` por step.
*   Runner casa por **major** (ou `"*"`).
*   Lock usa **CID** (verdade matemática).

> **Recomendação**: no authoring padrão, usar `"*"` (desenvolvimento) e gerar **lock** para produção.

### D) **Como lidar com `--vars`**

*   `--vars k=v` preenche **env inicial** (NRF via json\_view).
*   Vars **não** substituem `config` dos steps; são dados de entrada (tenant, priority, etc).

* * *

Diagrama lógico (alto nível)
----------------------------

```
product.json (+aliases)      modules.lock.json (opcional)
        │                             │
        ├─ parse/validate ────────────┤
        │                             │  (verifica CID por kind@major)
        └─ resolver aliases → pipeline normalizado
                      │
               CapRegistry (8 caps registradas)
                      │
               embed manifest + server
                      │
              ┌──────────────────────┐
              │     /evaluate        │  ← json_view → runner → recibos/metrics
              │     /health          │
              │     /version         │
              └──────────────────────┘
```

* * *

Estrutura de código sugerida
----------------------------

```
tools/ubl-cli/
  src/product/
    mod.rs
    schema.rs          # serde structs de product.json
    lock.rs            # schema do modules.lock.json
    aliases.rs         # map de aliases + normalização
    validate.rs        # validação autoral
    build.rs           # orquestra do comando from-manifest
    server.rs          # servidor HTTP do bin gerado
  src/cap/
    registry.rs        # preenchimento do CapRegistry (8 caps)
    versioning.rs      # match major/"*"
```

* * *

Pseudocódigo do comando
-----------------------

```rust
pub async fn from_manifest(args: Args) -> Result<()> {
    let product: Product = read_json(args.manifest)?;
    let mut product = aliases::normalize(product)?;
    validate::product(&product)?;

    let lock = if let Some(path) = args.lock {
        Some(read_json::<ModulesLock>(path)?)
    } else { None };

    if let Some(lock) = &lock {
        validate::lock(&product, lock)?; // confere kind@major → CID
    }

    let registry = cap::registry::build(lock.as_ref())?;
    let manifest_runner = schema::to_runner_manifest(&product, &registry)?;

    if args.dry_run {
        println!("{}", explain(&manifest_runner));
        return Ok(());
    }

    if args.container {
        return container::build(&product, &manifest_runner);
    }

    server::run(manifest_runner, registry, args).await
}
```

* * *

Regras de validação (mínimas e objetivas)
-----------------------------------------

*   **Pipeline**:
    *   `step_id` único.
    *   `kind` ∈ {cap-intake, cap-policy, cap-enrich, cap-transport, cap-permit, cap-pricing, cap-runtime, cap-llm} (ou alias resolvido).
    *   `version` = `"*"` **ou** major numérico (`"0"`, `"1"`…).
*   **Config** por `kind` (checagem leve, não precisa schema completo no MVP):
    *   intake: `map: [{from,to}]`.
    *   policy: `rules: [...]` (EXIST, THRESHOLD\_RANGE…).
    *   llm: `provider_hint` ∈ {"engine","smart"}, `model`, `max_tokens`.
    *   transport: `node`, `relay: []`.
    *   runtime: `allowed_ops`, `max_cpu_ms` etc.
*   **LOCK** (se presente):
    *   Todas as entradas `kind@major` do pipeline devem existir no `modules.lock`.
    *   Se `bytes` apontar `path:...`, calcular CID e **bater** com o CID do lock (senão → erro).

* * *

HTTP no bin gerado (shape final)
--------------------------------

*   **POST `/evaluate`**
    *   Body JSON → `env` NRF via json\_view.
    *   `Runner::run(&manifest_runner, env)` → coleta metrics, recibos, artifacts.
    *   Resposta com `{verdict, receipt_cid, url_rica, artifacts, metrics, stopped_at, product}`.
*   **GET `/health`** → 200 `{"ok":true}` (sem dependências externas).
*   **GET `/version`** → versão/git/ts.

### Erros (padronização)

*   `400` input inválido (mensagem curta + `error_id`).
*   `422` pipeline inválido (kind não registrado / versão).
*   `500` exceção inesperada (logar stack; resposta com `error_id`).

* * *

Container (flag `--container`)
------------------------------

*   **Dockerfile** multi-stage (Rust → scratch/distroless).
*   User não-root, `HEALTHCHECK`, `PORT` via env `PORT` (default 8787).
*   `docker run -p 8787:8787 ...` + `curl` → funciona.

* * *

Observabilidade (mínimo)
------------------------

*   Log estruturado por request: `{ req_id, cid, product, verdict, steps, latency_ms }`.
*   Métricas por step: `duration_ms`, contadores de efeitos (ex.: `runtime.request`).
*   _Correlation IDs_: propagar `X-Request-Id` se recebido.

* * *

Segurança (mínimo)
------------------

*   Limite de payload (ex.: 512KB) e tempo (timeout por request).
*   CORS opcional desativado por default.
*   **Sem segredos** em recibos/artifacts; sanitize de headers.

* * *

Critérios de aceite (Parte 2)
-----------------------------

1.  Dado um `product.json` válido, o comando:
    *   resolve aliases, valida pipeline, casa versão major/`"*"` e **(se lock)** verifica CIDs.
2.  Gera um bin que sobe `/evaluate` e roda **module-runner real** (runner-real).
3.  `--dry-run` imprime explicação do pipeline; `--container` entrega uma imagem funcional.
4.  Logs estruturados e códigos de erro padronizados.

* * *


Parte 3 — CLI & UX
==================

3.1. Comandos (UX de superfície)
--------------------------------

### 1) `ubl product from-manifest`

Gera e sobe o binário do produto (ou container).

```
ubl product from-manifest \
  --manifest ./products/tdln-policy/product.json \
  [--lock ./products/tdln-policy/modules.lock.json] \
  [--vars tenant=acme priority=normal ...] \
  [--port 8787] \
  [--container] \
  [--dry-run] \
  [--explain]
```

**Comportamento:**

*   Sem `--container`: sobe servidor local (`/evaluate`, `/health`, `/version`).
*   Com `--container`: produz imagem OCI (ou Dockerfile + build).
*   `--explain`: imprime o pipeline normalizado (pós-alias) e sai.
*   `--dry-run`: valida/esquematiza, mas não sobe servidor/gera imagem.

### 2) `ubl product lock`

Gera/atualiza **modules.lock.json** (pin por CID) a partir do manifesto.

```
ubl product lock \
  --manifest ./products/tdln-policy/product.json \
  --out ./products/tdln-policy/modules.lock.json \
  [--modules-path ./modules-build] \
  [--strict]
```

*   `--modules-path`: onde procurar bytes locais para calcular CIDs (opcional no MVP, útil em CI).
*   `--strict`: falha se algum `kind@major` do manifesto não tiver bytes mapeáveis (para produção).

### 3) `ubl product explain`

Somente imprime o pipeline expandido (aliases → kinds reais; validações aplicadas).

```
ubl product explain --manifest ./products/tdln-policy/product.json
```

* * *

3.2. Flags e mensagens (DX)
---------------------------

| Flag | Efeito | Mensagem/Erro |
| --- | --- | --- |
| `--manifest PATH` | Manifesto do produto | `error[4001]: product.json ausente ou inválido` |
| `--lock PATH` | Usar lock e verificar CIDs | `error[4202]: CID mismatch para cap-policy@0 (esperado X, calculado Y)` |
| `--vars k=v` | Injeta pares no env inicial | `warn[1009]: sobrescrevendo var 'tenant'` |
| `--explain` | Imprime pipeline normalizado | mostra JSON final do runner |
| `--dry-run` | Valida e sai | `ok: pipeline válido (N steps)` |
| `--container` | Gera imagem OCI | `ok: image built: ubl/product:tdln-policy` |
| `--port` | Porta do servidor | `info: listening on 0.0.0.0:8787` |

**Padrão de erro**:

```
{
  "error_id": "PRD-4202",
  "kind": "LockMismatch",
  "message": "CID mismatch para cap-policy@0",
  "hint": "rode: ubl product lock --manifest ... --out ...",
  "context": { "expected": "b3:...", "actual": "b3:..." }
}
```

* * *

3.3. Exemplos Copy-Paste (HELLO, WORLD!)
----------------------------------------

### A) Produto mínimo (policy)

**`products/tdln-policy/product.json`**

```json
{
  "v": 1,
  "name": "tdln-policy",
  "version": "1.0",
  "pipeline": [
    { "step_id": "step-0-cap-intake", "kind": "cap-intake", "version": "0",
      "config": { "map": [{ "from": "data", "to": "payload" }] } },
    { "step_id": "step-1-cap-policy", "kind": "cap-policy", "version": "0",
      "config": { "rules": [{ "EXIST": ["payload"] }] } },
    { "step_id": "step-2-cap-enrich", "kind": "cap-enrich", "version": "0",
      "config": { "providers": ["status_page"] } },
    { "step_id": "step-3-cap-transport", "kind": "cap-transport", "version": "0",
      "config": { "node": "did:ubl:local", "relay": [] } }
  ],
  "outputs": { "format": "json" }
}
```

**Subir local (sem container):**

```bash
ubl product from-manifest \
  --manifest ./products/tdln-policy/product.json \
  --vars tenant=acme priority=normal \
  --port 8787
```

**Chamar a API:**

```bash
curl -s -XPOST http://localhost:8787/evaluate \
  -H "Content-Type: application/json" \
  -d '{"data":"hello"}' | jq
```

**Saída esperada (resumo):**

```json
{
  "product":"tdln-policy",
  "verdict":"Allow",
  "receipt_cid":"b3:...",
  "url_rica":"https://resolver.local/r/b3:...",
  "metrics":[...],
  "artifacts":[...]
}
```

### B) LLM Engine (premium) e LLM Smart (local)

**`products/llm-engine/product.json`** (exemplo)

```json
{
  "v": 1,
  "name": "llm-engine",
  "version": "1.0",
  "pipeline": [
    { "step_id": "step-0-cap-intake", "kind": "cap-intake", "version": "0",
      "config": { "map": [{ "from": "prompt", "to": "llm.prompt" }] } },
    { "step_id": "step-1-policy.light", "kind": "cap-policy", "version": "0",
      "config": { "rules": [] } },
    { "step_id": "step-2-cap-intake", "kind": "cap-intake", "version": "0",
      "config": { "map": [{ "from": "tenant", "to": "meta.tenant" }] } },
    { "step_id": "step-3-cap-llm", "kind": "cap-llm", "version": "0",
      "config": {
        "provider_hint": "engine",
        "model": "gpt-4o-mini",
        "max_tokens": 4096
      } },
    { "step_id": "step-4-cap-enrich", "kind": "cap-enrich", "version": "0",
      "config": { "providers": ["status_page"] } },
    { "step_id": "step-5-cap-transport", "kind": "cap-transport", "version": "0",
      "config": { "node": "did:ubl:local", "relay": [] } }
  ],
  "outputs": { "format": "json" }
}
```

**Rodar:**

```bash
export OPENAI_API_KEY=sk-...
ubl product from-manifest --manifest ./products/llm-engine/product.json --port 8788
curl -s -XPOST http://localhost:8788/evaluate \
  -H "Content-Type: application/json" \
  -d '{"prompt":"hello"}' | jq
```

**`products/llm-smart/product.json`** (exemplo)

```json
{
  "v": 1,
  "name": "llm-smart",
  "version": "1.0",
  "pipeline": [
    { "step_id": "step-0-cap-intake", "kind": "cap-intake", "version": "0",
      "config": { "map": [{ "from": "prompt", "to": "llm.prompt" }] } },
    { "step_id": "step-1-policy.light", "kind": "cap-policy", "version": "0",
      "config": { "rules": [] } },
    { "step_id": "step-2-cap-intake", "kind": "cap-intake", "version": "0",
      "config": { "map": [{ "from": "priority", "to": "meta.prio" }] } },
    { "step_id": "step-3-cap-llm", "kind": "cap-llm", "version": "0",
      "config": {
        "provider_hint": "smart",
        "model": "ollama:llama3.1:8b",
        "max_tokens": 8192
      } },
    { "step_id": "step-4-cap-enrich", "kind": "cap-enrich", "version": "0",
      "config": { "providers": ["status_page"] } },
    { "step_id": "step-5-cap-transport", "kind": "cap-transport", "version": "0",
      "config": { "node": "did:ubl:local", "relay": [] } }
  ],
  "outputs": { "format": "json" }
}
```

**Rodar:**

```bash
export OLLAMA_BASE_URL=http://127.0.0.1:11434
ubl product from-manifest --manifest ./products/llm-smart/product.json --port 8789
curl -s -XPOST http://localhost:8789/evaluate \
  -H "Content-Type: application/json" \
  -d '{"prompt":"hello"}' | jq
```

* * *

3.4. Geração de LOCK (copy-paste)
---------------------------------

```bash
ubl product lock \
  --manifest ./products/tdln-policy/product.json \
  --out ./products/tdln-policy/modules.lock.json
```

**Validação com lock:**

```bash
ubl product from-manifest \
  --manifest ./products/tdln-policy/product.json \
  --lock ./products/tdln-policy/modules.lock.json \
  --port 8787
```

* * *

3.5. Container (copy-paste)
---------------------------

**Gerar imagem:**

```bash
ubl product from-manifest \
  --manifest ./products/tdln-policy/product.json \
  --container
# saída esperada: ok: image built: ubl/product:tdln-policy
```

**Rodar container:**

```bash
docker run --rm -p 8787:8787 \
  -e OPENAI_API_KEY \
  -e OLLAMA_BASE_URL \
  ubl/product:tdln-policy
```

* * *

3.6. Makefile & Scripts (DX)
----------------------------

**`Makefile` (trechos úteis):**

```make
PRODUCT?=tdln-policy
PORT?=8787

run:
	ubl product from-manifest --manifest ./products/$(PRODUCT)/product.json --port $(PORT)

explain:
	ubl product explain --manifest ./products/$(PRODUCT)/product.json

lock:
	ubl product lock --manifest ./products/$(PRODUCT)/product.json --out ./products/$(PRODUCT)/modules.lock.json

image:
	ubl product from-manifest --manifest ./products/$(PRODUCT)/product.json --container
```

**`scripts/bootstrap_product.sh` (Series 12):**

```bash
#!/usr/bin/env bash
set -euo pipefail

MANIFEST="$1"
LOCK_OUT="${2:-modules.lock.json}"

echo ">> validating $MANIFEST"
ubl product explain --manifest "$MANIFEST" >/dev/null

echo ">> generating lock at $LOCK_OUT"
ubl product lock --manifest "$MANIFEST" --out "$LOCK_OUT"

echo ">> done."
```

* * *

3.7. Mensagens UX (reais e úteis)
---------------------------------

*   **Sucesso (start):**
    ```
    ok: product tdln-policy ready on 0.0.0.0:8787
    manifest=v1 steps=4 lock=absent features=runner-real
    ```
*   **Explain:**
    ```
    pipeline:
      - step-0-cap-intake@0 { map: [ {from:data,to:payload} ] }
      - step-1-cap-policy@0  { rules: [ EXIST[payload] ] }
      - step-2-cap-enrich@0  { providers:[status_page] }
      - step-3-cap-transport@0 { node:did:ubl:local relay:[] }
    ```
*   **Erro de lock:**
    ```
    error[PRD-4202] CID mismatch for cap-llm@0
    expected=b3:abc... actual=b3:def...
    hint: re-gen lock (ubl product lock --manifest ... --out ...)
    ```

* * *

3.8. “Mão na massa” (guia de 5 minutos)
---------------------------------------

1.  **Crie a pasta do produto**
    ```bash
    mkdir -p products/tdln-policy
    ```
2.  **Cole o `product.json` (acima)**
3.  **Suba local**
    ```bash
    make run PRODUCT=tdln-policy PORT=8787
    ```
4.  **Teste com cURL**
    ```bash
    curl -s -XPOST http://localhost:8787/evaluate -H "Content-Type: application/json" -d '{"data":"hello"}' | jq
    ```
5.  **Gere o lock**
    ```bash
    make lock PRODUCT=tdln-policy
    ```
6.  **Gere imagem**
    ```bash
    make image PRODUCT=tdln-policy
    ```

* * *

3.9. Pitfalls (e como evitar)
-----------------------------

*   **Alias não resolvido**: sempre rode `ubl product explain` — se aparecer `policy.light`, algo ficou sem normalizar.
*   **Version major**: use `"0"` ou `"*"` — o runner casa por **major**; `1` não casa com módulo `0.x`.
*   **cap-intake mapping**: lembre que single key agora **insere** na raiz (bug fix feito); valide com `EXIST`.
*   **Providers LLM**: sem `OPENAI_API_KEY`/`OLLAMA_BASE_URL`, o LLM roda em modo stub (se compilado assim). Para modo real, exporte envs.
*   **Transport**: se configurar relay que depende de rede, erros aparecerão como `effect_error` — para MVP use `relay: []`.

* * *

3.10. Critérios de Aceite (Parte 3)
-----------------------------------

*   Usuário consegue, com os blocos acima, **subir um produto** em <5 min e receber `verdict`, `receipt_cid`, `url_rica`.
*   `explain`, `lock`, `container` funcionam exatamente como descrito.
*   Mensagens de erro são **autoexplicativas** e sempre trazem **hint**.

* * *


* * *

Parte 4 — Implementação (código)
================================

4.1. Layout de pastas (projeto atual)
-------------------------------------

```
tools/ubl-cli/
  src/
    main.rs
    modules.rs
    execute.rs
    product/
      mod.rs
      schema.rs         # Product {v,name,version,pipeline,outputs}
      aliases.rs        # policy.light → cap-policy, etc.
      translate.rs      # product.json → runner::Manifest
      lock.rs           # modules.lock.json (gen/verify por CID)
      server.rs         # HTTP server /evaluate /health /version
      container.rs      # (MVP) gerar Dockerfile / OCI
      explain.rs        # pipeline normalizado p/ stdout
```

> Dica: mantenha as dependências **do runner e dos módulos** atrás do feature `runner-real` (já existe): build rápido no default, real no `--features runner-real`.

* * *

4.2. Tipos centrais
-------------------

### 4.2.1. `Product` (schema do manifesto)

```rust
// tools/ubl-cli/src/product/schema.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub v: u32,
    pub name: String,
    pub version: String, // major ou "*"
    #[serde(default)]
    pub pipeline: Vec<Step>,
    #[serde(default)]
    pub outputs: Outputs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub step_id: String,
    pub kind: String,       // pode vir com alias (ex.: "policy.light")
    pub version: String,    // "0" ou "*"
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Outputs {
    #[serde(default = "default_format")]
    pub format: String,     // "json" | "capsule"
}
fn default_format() -> String { "json".into() }
```

### 4.2.2. `LockFile` (pin por CID)

```rust
// tools/ubl-cli/src/product/lock.rs
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockFile {
    pub product: String,
    pub product_version: String,
    pub modules: BTreeMap<String, LockedMod>, // key = "kind@major"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedMod {
    pub cid: String,        // "b3:..."
    pub bytes_len: u64,
    pub meta: Option<serde_json::Value>,
}
```

* * *

4.3. Aliases & Normalização
---------------------------

```rust
// tools/ubl-cli/src/product/aliases.rs
pub fn resolve_alias(kind: &str) -> &str {
    match kind {
        "policy.light" => "cap-policy",
        "cap-structure" => "cap-intake",
        "cap-llm-engine" => "cap-llm",
        "cap-llm-smart"  => "cap-llm",
        // add here conforme bundles
        k => k,
    }
}
```

* * *

4.4. Tradução → `runner::Manifest`
----------------------------------

```rust
// tools/ubl-cli/src/product/translate.rs
#[cfg(feature = "runner-real")]
pub fn to_runner_manifest(p: &Product) -> module_runner::manifest::Manifest {
    use module_runner::manifest::{Manifest, Step as RStep};
    Manifest {
        v: p.v,
        name: p.name.clone(),
        version: p.version.clone(),
        pipeline: p.pipeline.iter().map(|s| {
            RStep {
                step_id: s.step_id.clone(),
                kind: crate::product::aliases::resolve_alias(&s.kind).to_string(),
                version: s.version.clone(),
                config: s.config.clone(),
            }
        }).collect(),
    }
}
```

> Importante: mantenha **version** como `"0"` ou `"*"` (o `CapRegistry` já aceita `"*"` → casa qualquer major).

* * *

4.5. Lock: gerar e verificar (CID de bytes)
-------------------------------------------

### 4.5.1. Calcular CID (BLAKE3)

```rust
// tools/ubl-cli/src/product/lock.rs
use blake3::Hasher;

pub fn cid_of_bytes(bytes: &[u8]) -> String {
    let mut h = Hasher::new();
    h.update(bytes);
    format!("b3:{}", h.finalize().to_hex())
}
```

### 4.5.2. Gerar lock

```rust
// tools/ubl-cli/src/product/lock.rs
#[cfg(feature = "runner-real")]
pub fn generate_lock(p: &Product, resolver: &dyn BytesResolver) -> anyhow::Result<LockFile> {
    use std::collections::BTreeMap;
    let mut modules = BTreeMap::new();
    for s in &p.pipeline {
        let kind = crate::product::aliases::resolve_alias(&s.kind);
        let key = format!("{}@{}", kind, major_of(&s.version)?);
        if modules.contains_key(&key) { continue; }

        let bytes = resolver.bytes_for(kind)?; // ex.: retorna o .rlib/.wasm do módulo
        let cid = cid_of_bytes(&bytes);
        modules.insert(key, LockedMod { cid, bytes_len: bytes.len() as u64, meta: None });
    }
    Ok(LockFile { product: p.name.clone(), product_version: p.version.clone(), modules })
}

fn major_of(v: &str) -> anyhow::Result<u32> {
    if v == "*" { return Ok(0) } // convenção: * → bucket 0 (não importa)
    let major = v.split('.').next().unwrap_or("0").parse::<u32>().unwrap_or(0);
    Ok(major)
}

/// Abstração para encontrar bytes (MVP: lookup em map estático/paths)
pub trait BytesResolver {
    fn bytes_for(&self, kind: &str) -> anyhow::Result<Vec<u8>>;
}
```

### 4.5.3. Verificar lock na execução

```rust
// tools/ubl-cli/src/product/lock.rs
#[cfg(feature = "runner-real")]
pub fn verify_lock(p: &Product, lock: &LockFile, resolver: &dyn BytesResolver) -> anyhow::Result<()> {
    for s in &p.pipeline {
        let kind = crate::product::aliases::resolve_alias(&s.kind);
        let key = format!("{}@{}", kind, major_of(&s.version)?);

        let locked = lock.modules.get(&key)
            .ok_or_else(|| anyhow::anyhow!("lock missing key {}", key))?;

        let bytes = resolver.bytes_for(kind)?;
        let actual = cid_of_bytes(&bytes);
        if locked.cid != actual {
            anyhow::bail!("CID mismatch for {} (expected {}, got {})", key, locked.cid, actual);
        }
    }
    Ok(())
}
```

> **Resolver** no MVP pode mapear de **kind → bytes embutidos** (build.rs/`include_bytes!`) ou **artefatos `.wasm`/`.rlib`** produzidos no pipeline de build. O objetivo é _pin por math_.

* * *

4.6. Execução do Produto (server)
---------------------------------

```rust
// tools/ubl-cli/src/product/server.rs
#[cfg(feature = "runner-real")]
pub async fn serve_product(p: Product, addr: std::net::SocketAddr) -> anyhow::Result<()> {
    use axum::{Router, routing::post, extract::State, Json};
    use serde_json::Value as JsonValue;
    use module_runner::{Runner, executors::LoggingExecutor};

    let manifest = crate::product::translate::to_runner_manifest(&p);
    let registry = crate::execute::build_cap_registry()?; // já existe
    let executor = LoggingExecutor::default();            // trocar p/ DispatchExecutor quando “live”
    let runner = Runner::new(registry, executor);

    #[derive(Clone)]
    struct AppState {
        manifest: module_runner::manifest::Manifest,
        runner: Runner,
    }

    let app = Router::new()
        .route("/evaluate", post(handle_eval))
        .route("/health", post(|| async { "ok" }))
        .with_state(AppState { manifest, runner });

    axum::Server::bind(&addr).serve(app.into_make_service()).await?;
    Ok(())
}

#[cfg(feature = "runner-real")]
async fn handle_eval(
    State(state): State<crate::product::server::AppState>,
    Json(input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    // NRF <-> JSON view
    let env = ubl_json_view::from_json(&input).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    let out = state.runner.run(&state.manifest, env).await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    let json = ubl_json_view::to_json(&out.env); // resultado final
    Ok(Json(json))
}
```

> **/version**: responda `{ version, git_sha, build_ts }`. **/health**: “ok”.

* * *

4.7. CLI (subcomandos)
----------------------

```rust
// tools/ubl-cli/src/main.rs (trecho)
match args.cmd {
  Cmd::Product(ProductCmd::FromManifest { manifest, lock, vars, port, container, dry_run, explain }) => {
    let p: Product = read_json(manifest)?;
    if let Some(lockp) = lock { verify_lock(&p, &read_lock(lockp)?, &resolver)?; }
    if explain { product::explain::print(&p)?; return Ok(()); }
    if dry_run { product::explain::validate(&p)?; return Ok(()); }
    if container { product::container::build(&p)?; return Ok(()); }

    let addr = format!("0.0.0.0:{}", port).parse()?;
    product::server::serve_product(p, addr).await?;
  }
  Cmd::Product(ProductCmd::Lock { manifest, out, modules_path, strict }) => {
    let p: Product = read_json(manifest)?;
    let lock = product::lock::generate_lock(&p, &resolver_from(modules_path))?;
    write_json(out, &lock)?;
  }
  Cmd::Product(ProductCmd::Explain { manifest }) => {
    let p: Product = read_json(manifest)?;
    product::explain::print(&p)?;
  }
  _ => {}
}
```

* * *

4.8. Container (MVP)
--------------------

```rust
// tools/ubl-cli/src/product/container.rs
pub fn build(p: &Product) -> anyhow::Result<()> {
    let dockerfile = format!(r#"
FROM gcr.io/distroless/cc
ENV RUST_LOG=info
COPY ./bin/product /app/product
EXPOSE 8787
ENTRYPOINT ["/app/product", "--port", "8787"]
"#);
    std::fs::write("Dockerfile.product", dockerfile)?;
    // MVP: imprimir instrução de build
    eprintln!("run: docker build -f Dockerfile.product -t ubl/product:{} .", p.name);
    Ok(())
}
```

> Em seguida, podemos acoplar o build real (usar `cargo build --bin product` gerado no target out-of-tree, caso opte por _codegen_ de um bin mínimo).

* * *

4.9. Testes de fumaça (CLI)
---------------------------

```rust
// tools/ubl-cli/tests/product_smoke.rs
#[cfg(feature = "runner-real")]
#[tokio::test]
async fn product_policy_allows() {
    use ubl_cli::product::{schema::Product, translate::to_runner_manifest};
    let p: Product = serde_json::from_str(include_str!("../fixtures/tdln-policy.product.json")).unwrap();
    let m = to_runner_manifest(&p);
    // Cria runner in-process
    let reg = ubl_cli::execute::build_cap_registry().unwrap();
    let exec = module_runner::executors::LoggingExecutor::default();
    let runner = module_runner::Runner::new(reg, exec);

    let env = serde_json::json!({"data":"hello"});
    let env = ubl_json_view::from_json(&env).unwrap();
    let out = runner.run(&m, env).await.unwrap();
    let res = ubl_json_view::to_json(&out.env);
    assert_eq!(res.get("verdict").and_then(|v| v.as_str()), Some("Allow"));
}
```

> Coloque **fixtures** em `tools/ubl-cli/fixtures/*.product.json`.

* * *

4.10. CI (jobs úteis)
---------------------

*   **Build** (default): `cargo check --workspace`
*   **Runner-real**: `cargo build -p ubl-cli --features runner-real`
*   **Fumaça**: `cargo test -p ubl-cli --features runner-real --tests product_smoke`
*   **Locks**: job que roda `ubl product lock` e valida CID (falha em mismatch)
*   **Container** (opcional): build de imagem com tag `ubl/product:${PRODUCT}`

* * *

4.11. Segurança & Canônico (essenciais)
---------------------------------------

*   **CID-bytes**: lock sempre por **bytes** do módulo (não por “versão”). 🔐
*   **WASM sandbox**: quando o `cap-runtime` receber código do cliente, execute-o no **Certified Runtime** (WASI, policies). Exponha métrica `runtime.request` e artefatos com o **attestation**.
*   **Determinismo estrutural**: mesmo `product.json` + mesmo `modules.lock.json` ⇒ **mesmo plano** (mesmo que LLM varie).
*   **RICA**: garanta que o `cap-enrich` gere a status page e o `cap-transport` anexe a cadeia de recibos; `/r/{cid}` deve resolver (implementar no _registry_).

* * *

4.12. Extensões (logo após o MVP)
---------------------------------

*   **Cache por CID** (LLM): chave = `blake3(nrf_bytes_de_explicacao)` ⇒ cai custo e latência 😎
*   **Bundles**: `includes: ["llm-engine-bundle"]` → pré-expansão no `explain.rs`
*   **Profiles**: `--profile dev|prod` com defaults (latência, modelo, logs)
*   **Observabilidade**: métricas por step (`duration_ms`, `rules_failed`, etc.) já saem nos artefatos — somar `--jsonl` para stream de execuções.

* * *

4.13. Critérios de aceite (Parte 4)
-----------------------------------

*   `ubl product from-manifest` sobe **HTTP** e responde `/evaluate` com **receipt\_cid** e **url\_rica**.
*   `ubl product lock` gera **modules.lock.json** e `verify_lock` barra mismatch.
*   `explain` imprime pipeline normalizado (aliases resolvidos).
*   Teste de fumaça (policy) **passa** com `Allow` usando `runner-real`.

* * *


* * *

Parte 5 — Exemplos, Fixtures, Scripts & QA
==========================================

5.1. Estrutura recomendada (pasta do repo)
------------------------------------------

```
tools/ubl-cli/
  fixtures/
    products/
      tdln-policy.product.json
      tdln-runtime.product.json
      llm-engine.product.json
      llm-smart.product.json
    policies/
      eu-ai-act.yaml
      demo-allow-on-payload.yaml
    suites/
      bench-base.yaml
  scripts/
    product_bootstrap.sh
    product_lock.sh
    product_smoke.sh
    product_docker_build.sh
    product_k8s_apply.sh
  tests/
    product_smoke.rs
```

* * *

5.2. Manifestos **prontos** (4 produtos)
----------------------------------------

> Coloque em `tools/ubl-cli/fixtures/products/*.product.json`

### 5.2.1. `tdln-policy.product.json` (puro com políticas → recibo + URL)

```json
{
  "v": 1,
  "name": "tdln-policy",
  "version": "1",
  "pipeline": [
    {
      "step_id": "step-0-cap-intake",
      "kind": "cap-intake",
      "version": "0",
      "config": {
        "mappings": [
          {"to": "payload", "from": "data"}
        ]
      }
    },
    {
      "step_id": "step-1-policy.light",
      "kind": "policy.light",
      "version": "0",
      "config": {
        "rules": [
          {"kind":"EXIST","path":["payload"]}
        ]
      }
    },
    {
      "step_id": "step-2-cap-enrich",
      "kind": "cap-enrich",
      "version": "0",
      "config": {
        "providers": [{ "kind": "status-page" }]
      }
    },
    {
      "step_id": "step-3-cap-transport",
      "kind": "cap-transport",
      "version": "0",
      "config": {
        "node": "did:rica:local",
        "relay": []
      }
    }
  ],
  "outputs": { "format": "json" }
}
```

### 5.2.2. `tdln-runtime.product.json` (puro com Certified Runtime no meio)

```json
{
  "v": 1,
  "name": "tdln-runtime",
  "version": "1",
  "pipeline": [
    { "step_id":"step-0-cap-intake", "kind":"cap-intake", "version":"0",
      "config": { "mappings":[ {"to": "payload", "from": "sourceData"} ] } },
    { "step_id":"step-1-cap-runtime", "kind":"cap-runtime", "version":"0",
      "config": { "code_path":"userCode", "timeout_ms":2000, "wasi":true } },
    { "step_id":"step-2-policy.light", "kind":"policy.light", "version":"0",
      "config": { "rules":[ {"kind":"EXIST", "path":["runtime","attestation"]} ] } },
    { "step_id":"step-3-cap-enrich", "kind":"cap-enrich", "version":"0",
      "config": { "providers":[ {"kind":"status-page"} ] } },
    { "step_id":"step-4-cap-transport", "kind":"cap-transport", "version":"0",
      "config": { "node":"did:rica:local", "relay":[] } }
  ],
  "outputs": { "format": "json" }
}
```

### 5.2.3. `llm-engine.product.json` (bundle premium; tdln ajuda o prompt)

```json
{
  "v": 1,
  "name": "llm-engine",
  "version": "1",
  "pipeline": [
    { "step_id":"step-0-cap-intake", "kind":"cap-intake", "version":"0",
      "config": { "mappings":[ {"to":"prompt","from":"prompt"} ] } },
    { "step_id":"step-1-policy.light", "kind":"policy.light", "version":"0",
      "config": { "rules":[ {"kind":"EXIST","path":["prompt"]} ] } },
    { "step_id":"step-2-cap-intake", "kind":"cap-intake", "version":"0",
      "config": { "mappings":[ {"to":"prompt_structured","from":"prompt"} ] } },
    { "step_id":"step-3-cap-llm", "kind":"cap-llm", "version":"0",
      "config": {
        "provider":"engine",
        "model":"gpt-4o-mini",
        "max_tokens":4096,
        "latency_budget_ms":1500
      } },
    { "step_id":"step-4-cap-enrich", "kind":"cap-enrich", "version":"0",
      "config": { "providers":[ {"kind":"status-page"} ] } },
    { "step_id":"step-5-cap-transport", "kind":"cap-transport", "version":"0",
      "config": { "node":"did:rica:local", "relay":[] } }
  ],
  "outputs": { "format": "json" }
}
```

### 5.2.4. `llm-smart.product.json` (bundle local/8B; custo ≈ zero)

```json
{
  "v": 1,
  "name": "llm-smart",
  "version": "1",
  "pipeline": [
    { "step_id":"step-0-cap-intake", "kind":"cap-intake", "version":"0",
      "config": { "mappings":[ {"to":"prompt","from":"prompt"} ] } },
    { "step_id":"step-1-policy.light", "kind":"policy.light", "version":"0",
      "config": { "rules":[ {"kind":"EXIST","path":["prompt"]} ] } },
    { "step_id":"step-2-cap-intake", "kind":"cap-intake", "version":"0",
      "config": { "mappings":[ {"to":"prompt_structured","from":"prompt"} ] } },
    { "step_id":"step-3-cap-llm", "kind":"cap-llm", "version":"0",
      "config": {
        "provider":"smart",
        "model":"ollama:llama3.1:8b",
        "max_tokens":8192,
        "latency_budget_ms":2500
      } },
    { "step_id":"step-4-cap-enrich", "kind":"cap-enrich", "version":"0",
      "config": { "providers":[ {"kind":"status-page"} ] } },
    { "step_id":"step-5-cap-transport", "kind":"cap-transport", "version":"0",
      "config": { "node":"did:rica:local", "relay":[] } }
  ],
  "outputs": { "format": "json" }
}
```

* * *

5.3. Scripts (MVP)
------------------

> Coloque em `tools/ubl-cli/scripts/*.sh` (dar `chmod +x`)

### 5.3.1. `product_bootstrap.sh`

Gera lock e valida, imprime plano (explain) e sobe local.

```bash
#!/usr/bin/env bash
set -euo pipefail
MANIFEST="${1:?usage: $0 <product.json>}"
LOCK="${2:-modules.lock.json}"
PORT="${PORT:-8787}"

# lock
ubl product lock --manifest "$MANIFEST" --out "$LOCK" --modules-path ./target/modules

# explain
ubl product explain --manifest "$MANIFEST"

# serve
UBL_LOG=info ubl product from-manifest --manifest "$MANIFEST" --lock "$LOCK" --port "$PORT" --features runner-real
```

### 5.3.2. `product_lock.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
MANIFEST="${1:?usage: $0 <product.json>}"
LOCK="${2:-modules.lock.json}"
ubl product lock --manifest "$MANIFEST" --out "$LOCK" --modules-path ./target/modules
```

### 5.3.3. `product_smoke.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
PORT="${PORT:-8787}"
JSON="${1:-'{"data":"hello"}'}"
curl -sS -H 'Content-Type: application/json' -d "$JSON" "http://127.0.0.1:${PORT}/evaluate" | jq .
```

### 5.3.4. `product_docker_build.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
MANIFEST="${1:?usage: $0 <product.json>}"
NAME="$(jq -r .name "$MANIFEST")"
ubl product from-manifest --manifest "$MANIFEST" --container
docker build -f Dockerfile.product -t "ubl/product:${NAME}" .
```

### 5.3.5. `product_k8s_apply.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
NAME="${1:?usage: $0 <product-name>}"
cat <<YAML | kubectl apply -f -
apiVersion: apps/v1
kind: Deployment
metadata: { name: ${NAME} }
spec:
  replicas: 1
  selector: { matchLabels: { app: ${NAME} } }
  template:
    metadata: { labels: { app: ${NAME} } }
    spec:
      containers:
      - name: ${NAME}
        image: ubl/product:${NAME}
        ports: [{containerPort: 8787}]
        env:
        - { name: RUST_LOG, value: "info" }
---
apiVersion: v1
kind: Service
metadata: { name: ${NAME} }
spec:
  selector: { app: ${NAME} }
  ports: [{ port: 80, targetPort: 8787 }]
YAML
```

* * *

5.4. Como rodar local (passo a passo)
-------------------------------------

1.  **Build** (pipeline real):

```
cargo build -p ubl-cli --features runner-real
```

2.  **Subir tdln-policy**:

```
tools/ubl-cli/scripts/product_bootstrap.sh tools/ubl-cli/fixtures/products/tdln-policy.product.json
```

3.  **Testar**:

```
PORT=8787 tools/ubl-cli/scripts/product_smoke.sh '{"data":"hello"}'
```

4.  **Ver saída esperada**: `verdict: "Allow"`, `receipt_cid`, `url_rica`, `receipt_chain`.
5.  **LLM Engine/Smart**:

```
OPENAI_API_KEY=... ./target/debug/ubl product from-manifest --manifest tools/ubl-cli/fixtures/products/llm-engine.product.json --port 8788
OLLAMA_BASE_URL=http://127.0.0.1:11434 ./target/debug/ubl product from-manifest --manifest tools/ubl-cli/fixtures/products/llm-smart.product.json --port 8789
```

* * *

5.5. CURLs úteis
----------------

```
# Evaluate (POST)
curl -sS -H 'Content-Type: application/json' \
  -d '{"data":"hello"}' http://127.0.0.1:8787/evaluate | jq

# Health
curl -sS http://127.0.0.1:8787/health

# (Registry) Resolver raw vs HTML (quando estiver implementado no serviço registry)
curl -sS -H 'Accept: application/ai-nrf1' http://registry/r/b3:<CID>
curl -sS -H 'Accept: text/html'          http://registry/r/b3:<CID>
```

* * *

5.6. CI (GitHub Actions — sugestões de jobs)
--------------------------------------------

*   **build-default**  
    `cargo check --workspace`
*   **build-runner-real**  
    `cargo build -p ubl-cli --features runner-real`
*   **smoke-products**  
    `cargo test -p ubl-cli --features runner-real --tests product_smoke`
*   **lock-verify**  
    Rodar `ubl product lock` e comparar com lock existente (falhar em mismatch).
*   **container** (opcional)  
    Build `Dockerfile.product` para cada product fixture.

* * *

5.7. Qualidade & Hardening (checklist)
--------------------------------------

**Determinismo estrutural**

*    `product.json` + `modules.lock.json` → mesmo plano sempre.
*    `CapRegistry` aceita `"*"` mas _preferir_ travar major quando estabilizar.

**Pin por math**

*    `modules.lock.json` guarda `cid` BLAKE3 dos bytes do módulo.
*    Execução verifica CID antes de rodar.

**WASM / Certified Runtime**

*    `cap-runtime` executa WASM com WASI, tempo máx (`timeout_ms`) e sandbox.
*    Emite `runtime.attestation` (inclua no `policy.light` como regra).

**Transparência**

*    `cap-enrich` gera status page (artefato).
*    `cap-transport` agrega os recibos por hop → `receipt_chain`, `url_rica`.

**Observabilidade**

*    Métricas por step: `duration_ms`, `rules_failed`, `runtime.request`, etc.
*    Log estruturado (JSON) no server.

**Segurança**

*    Sem segredo em logs/artefatos (redact simples).
*    Cabeçalhos CORS/Rate-limit no edge quando publicar.

* * *

5.8. Troubleshooting rápido
---------------------------

*   **`invalid cap-transport config`**  
    Ajuste `node` (ex.: `did:rica:local`) e `relay` (pode ser `[]` no MVP).
*   **`cap not found: cap-llm 1.x`**  
    Versão do manifesto não casa com major do módulo. Use `"version":"0"` ou `"*"`.
*   **`Cannot start a runtime from within a runtime`**  
    Evite `block_on` dentro de `#[tokio::main]`. A API já está assíncrona.
*   **Policy DENY inesperado**  
    Verifique `cap-intake` mappings (com o bug fix, single key insere no root).

* * *

5.9. Próximos passos (rápidos)
------------------------------

1.  **Resolver `/r/{cid}` no registry** (negociação de `Accept`).
2.  **Cache por CID no `cap-llm`** (explicações determinísticas baratas).
3.  **`ubl product init`** (scaffold de produto + lock automático).
4.  **`tdln bench-report`** (comparar Engine vs Smart com suite pronta).

* * *

5.10. Mini-README para cada produto (copiar e colar)
----------------------------------------------------

**Run (local):**

```
cargo build -p ubl-cli --features runner-real
UBL_LOG=info ./target/debug/ubl product from-manifest \
  --manifest tools/ubl-cli/fixtures/products/<NAME>.product.json \
  --port 8787
```

**Evaluate:**

```
curl -sS -H 'Content-Type: application/json' -d '{"data":"hello"}' \
  http://127.0.0.1:8787/evaluate | jq
```

**Lock:**

```
./tools/ubl-cli/scripts/product_lock.sh tools/ubl-cli/fixtures/products/<NAME>.product.json
```

**Docker:**

```
./tools/ubl-cli/scripts/product_docker_build.sh tools/ubl-cli/fixtures/products/<NAME>.product.json
```

* * *


Objetivo: conectar a CLI ao module-runner definitivo.
Entregáveis:

Registro das 8 capabilities no CapRegistry.

Tradução de aliases → kinds reais.

Semântica de version (major/*) + validação contra lock.

Execução assíncrona e mapeamento de métricas/recibos para a resposta HTTP.
Aceite: smoke E2E com 4 produtos canônicos (tdln-policy, tdln-runtime, llm-engine, llm-smart).


Visão geral
===========

Conectar a CLI (`ubl`) ao **module-runner** definitivo, de forma **assíncrona**, validando **version/lock**, traduzindo **aliases → kinds reais** e retornando **métricas + recibos** corretamente na **resposta HTTP**.

* * *

1) CapRegistry: registrar as 8 capabilities
-------------------------------------------

**Entregável:** todas as caps disponíveis via `CapRegistry::register(Box<dyn Capability>)`.

**Checklist**

*    `cap-intake`
*    `cap-policy` (apelidos: `policy.light`, `policy.hardening`)
*    `cap-enrich`
*    `cap-transport`
*    `cap-permit` (se aplicável ao produto)
*    `cap-llm` (providers: `engine`, `smart`)
*    `cap-pricing` (se aplicável)
*    `cap-runtime` (WASM/WASI)

**Esqueleto**

```rust
let mut reg = CapRegistry::default();
reg.register(Box::new(cap_intake::IntakeModule::default()));
reg.register(Box::new(cap_policy::PolicyModule::default()));
reg.register(Box::new(cap_enrich::EnrichModule::default()));
reg.register(Box::new(cap_transport::TransportModule::default()));
reg.register(Box::new(cap_permit::PermitModule::default()));
reg.register(Box::new(cap_llm::LlmModule::default()));        // provider_hint via config
reg.register(Box::new(cap_pricing::PricingModule::default()));
reg.register(Box::new(cap_runtime::RuntimeModule::default()));
```

**Aceite local**

*   `reg.has("cap-llm") == true`, etc.
*   `reg.resolve(kind="cap-llm", version="0" | "*")` retorna módulo.

* * *

2) Tradução de aliases → kinds reais
------------------------------------

**Entregável:** camada de normalização que converte manifests “ergonômicos” para o formato do runner.

**Alias table (exemplos)**

*   `policy.light` → `cap-policy`
*   `policy.hardening` → `cap-policy`
*   `cap-structure` → `cap-intake`
*   `cap-llm-engine` → `cap-llm` + `config.provider="engine"`
*   `cap-llm-smart` → `cap-llm` + `config.provider="smart"`

**Função sugerida**

```rust
fn normalize_step(mut step: RunnerStep) -> RunnerStep {
    match step.kind.as_str() {
        "policy.light" | "policy.hardening" => { step.kind = "cap-policy".into(); step }
        "cap-structure" => { step.kind = "cap-intake".into(); step }
        "cap-llm-engine" => { step.kind = "cap-llm".into(); step.config["provider"] = "engine".into(); step }
        "cap-llm-smart"  => { step.kind = "cap-llm".into(); step.config["provider"] = "smart".into(); step }
        _ => step,
    }
}
```

**Aceite local**

*   Manifesto YAML/JSON com `policy.light` vira runner-step `cap-policy`.
*   `llm-engine` e `llm-smart` ajustam `provider` automaticamente.

* * *

3) Semântica de version (major / `*`) + validação contra lock
-------------------------------------------------------------

**Entregável:** seleção de módulo por **major** e “curinga” (`*`), com **validação por lock**.

**Regra**

*   Se `version == "*"`, aceita qualquer major instalado (útil enquanto estabiliza).
*   Se `version == "0"` (ou `"1"`, etc.), só aceita módulos com **major** correspondente.
*   **Antes de executar**, validar **`modules.lock.json`**:
    *   Para cada step: (kind, major) devem existir **e** o **CID** do módulo (bytes) deve bater com o lock.
    *   Falhar hard se mismatch (princípio _pinned by math_).

**Pseudocódigo**

```rust
fn resolve_module(kind: &str, req_ver: &str, lock: &Lock) -> anyhow::Result<ModuleRef> {
    let m = registry.resolve(kind, req_ver)?;       // semver major/* match
    let bytes_cid = blake3_of(m.bytes())?;
    let locked = lock.lookup(kind).ok_or_else(|| err!("not locked"))?;
    ensure!(locked.cid == bytes_cid, "locked CID mismatch for {kind}");
    Ok(m)
}
```

**Aceite local**

*   `version:"*"` funciona, mas emite **warning** se lock pede major específico.
*   Mismatch de CID → erro explícito: `locked CID mismatch for cap-llm`.

* * *

4) Execução assíncrona (Tokio) e mapeamento de métricas/recibos
---------------------------------------------------------------

**Entregável:** `run_manifest_async(...)` chamando `Runner::run` e consolidando:

*   `metrics` (por step)
*   `receipt_chain` (CIDs em ordem)
*   `receipt_cid` (primeiro hop, identidade do caso)
*   `stopped_at` (se houve DENY)
*   `url_rica` (resolver do registry)
*   `verdict` (Allow/Deny)

**Assinatura sugerida**

```rust
pub async fn run_manifest(
    manifest: RunnerManifest,
    env: nrf_core::Value,
    registry: &CapRegistry,
    opts: ExecOpts,
) -> anyhow::Result<HttpResult>; // HttpResult mapeia para a resposta JSON
```

**Estrutura de resposta HTTP**

```json
{
  "product": "tdln-policy",
  "ok": true,
  "verdict": "Allow",
  "stopped_at": null,
  "receipt_cid": "b3:...",
  "receipt_chain": ["b3:...", "b3:...", "..."],
  "url_rica": "https://resolver.local/r/b3:...",
  "metrics": [
    {"step":"step-0-cap-intake","metric":"duration_ms","value":0},
    {"step":"step-1-cap-policy","metric":"rules_failed","value":0},
    ...
  ],
  "artifacts": 2
}
```

**Detalhes**

*   **Primeiro CID** da cadeia como `receipt_cid` (âncora pública).
*   `url_rica` deriva de `receipt_cid` + base configurável.
*   `metrics` vêm do `Effect::Metric` ou medição local de cada step.
*   **Transport** soma hop atual + anteriores; não precisa I/O real no MVP (usar executor que apenas encadeia recibos — _Logging/Simulated_).

* * *

5) Smoke E2E (4 produtos canônicos)
-----------------------------------

**Entregável:** todos executam **Allow** end-to-end com `--features runner-real`.

**Comandos**

```bash
# build
cargo build -p ubl-cli --features runner-real

# tdln-policy
./target/debug/ubl tdln policy --var data=hello

# tdln-runtime
./target/debug/ubl tdln runtime --var sourceData=test --var userCode=print

# llm-engine  (requer OPENAI_API_KEY/ANTHROPIC_API_KEY se live; no MVP pode simular)
./target/debug/ubl llm engine --var prompt=hello

# llm-smart   (requer OLLAMA_BASE_URL se live; no MVP pode simular)
./target/debug/ubl llm smart --var prompt=hello
```

**Critérios de Aceite**

*   **Todos** retornam `ok:true`, `verdict:"Allow"`, `stopped_at:null`.
*   `receipt_chain` com N hops (intake → policy → \[runtime|llm\] → enrich → transport).
*   `metrics` presentes por step (incl. `rules_failed` na policy, `runtime.request` na runtime, `prompt_len` e `max_tokens` no llm).
*   `url_rica` formatado.
*   Quando **policy** deve negar (teste controlado), `verdict:"Deny"`, `stopped_at:"step-1-cap-policy"` e **sem** steps subsequentes.

* * *

6) Erros & mapeamento HTTP
--------------------------

*   **400** Manifesto inválido / schema de config incorreto.
*   **409** Mismatch de lock (CID não confere).
*   **422** Policy Deny é **200** com `ok:true` + `verdict:"Deny"` (faz parte do fluxo de negócio, não é erro técnico).
*   **500** Falhas internas inesperadas (panic, I/O real quando habilitar).

* * *

7) Feature flags & modos de execução
------------------------------------

*   `--features runner-real` → usa `module-runner` real + `CapRegistry` completo.
*   Sem feature → **stubs** rápidos para DX (compila e roda help/validate/explain).
*   `--in-process` (default) vs `--registry URL` (futuro): mesmo plano, mudança só no executor.

* * *

8) CI recomendado (Jobs)
------------------------

*   **check**: `cargo check --workspace`
*   **build-runner-real**: `cargo build -p ubl-cli --features runner-real`
*   **smoke-products**: rodar 4 comandos acima (com providers simulados) e validar `verdict`, `receipt_chain.len()`, `metrics`.
*   **lock-verify**: `ubl product lock --manifest ...` e `diff` com lock comitado.

* * *

9) Pontos de atenção (falhas comuns)
------------------------------------

*   **`cap not found: cap-llm 1.0`** → use `"version":"0"` ou `"*"` enquanto o major for 0.
*   **`invalid cap-transport config`** → exigir `node: "did:rica:local"` e `relay: []` no MVP.
*   **Tokio nested** → tudo **async**; nunca `block_on` dentro de `#[tokio::main]`.
*   **intake single key** → bug já corrigido: inserção de chave na raiz, não overwrite.

* * *

10) Done = Definition of Ready (DoR) / Done (DoD)
-------------------------------------------------

**DoR**

*    Alias map definido e coberto por testes de unidade (tradução 1:1).
*    Parser de manifest YAML/JSON → runner manifest.

**DoD**

*    CapRegistry com 8 módulos
*    Version semantics (major/\*) + **validação por lock/CID**
*    Execução **async**, resposta HTTP com métricas/recibos/url\_rica
*    **Smoke E2E**: tdln-policy, tdln-runtime, llm-engine, llm-smart (Allow)
*    CI com `build-runner-real` + `smoke-products`

* * *


Objetivo: shape do servidor contido no bin gerado.
Entregáveis:

Rotas (/evaluate, /health, /version), middlewares (limite de payload, timeout, CORS).

Mapeamento JSON→NRF (json_view), tratamento de erros, códigos HTTP.

Opções: porta, logs estruturados, graceful shutdown.
Aceite: curl simples retorna receipt_cid + url_rica.


Objetivo
========

O binário gerado por `ubl product from-manifest` sobe um **servidor HTTP** mínimo, que recebe JSON, transforma em **NRF (json\_view)**, executa o **pipeline** (Runner) e responde com **recibo** (CID + URL RICA), métricas e status.

* * *

1) Rotas
========

**`POST /evaluate`**

*   **Body**: JSON arbitrário do cliente (ex.: `{ "payload": "...", ... }`)
*   **Headers aceitos**:
    *   `X-Product: <name>` (opcional; default: nome do manifest)
    *   `Accept: application/json` (default)
*   **Resposta 200** (negação é negócio, **não** erro):
    ```json
    {
      "product": "tdln-policy",
      "ok": true,
      "verdict": "Allow|Deny",
      "stopped_at": null | "step-<id>",
      "receipt_cid": "b3:<...>",
      "receipt_chain": ["b3:<...>", "..."],
      "url_rica": "https://resolver.local/r/b3:<...>",
      "metrics": [{ "step":"...", "metric":"...", "value": 0 }],
      "artifacts": 0
    }
    ```
*   **Erros**:
    *   `400` JSON inválido / schema incorreto do manifest params
    *   `409` **lock mismatch** (CID do módulo ≠ lock)
    *   `500` falha interna inesperada

**`GET /health`**

*   `200 OK` com `{ "status": "ok" }` (pode incluir checks leves)

**`GET /version`**

*   `200 OK` com `{ "version": "<semver>", "git_sha": "<sha>", "build_ts": "<rfc3339>" }`

* * *

2) Middlewares (tower)
======================

*   **Limite de payload**: 1–2MB (configurável)
*   **Timeout por requisição**: ex. 5s/15s (configurável)
*   **CORS**: permissivo por default (origens configuráveis)
*   **Request-ID**: gera `req_id` p/ logs e propaga em `X-Request-Id`
*   **Compression**: gzip/deflate/br (automático)
*   **Rate limit** (opcional, por IP/chave)

* * *

3) Mapeamento JSON → NRF (json\_view)
=====================================

1.  Ler corpo JSON (`serde_json::Value`).
2.  Converter p/ **NRF**: `ubl_json_view::from_json(&json) -> nrf_core::Value`.
3.  Injetar **vars** do produto/CLI (porta, tenant, etc) no env NRF se necessário.
4.  Executar `Runner::run(...)` com o **manifest traduzido** + **CapRegistry**.
5.  Converter métricas/saída p/ JSON (view reversa) p/ resposta HTTP.

> Observação: já corrigimos o caso _single-key_ no `cap-intake` — o env não é mais sobrescrito, a chave é inserida.

* * *

4) Tratamento de erros e códigos HTTP
=====================================

**Convenção**

*   **Deny** de policy **não é erro**: retorna `200` com `verdict:"Deny"` e `stopped_at`.
*   Erros de entrada/config → `400` (`InvalidInput`).
*   Lock mismatch (CID) → `409` (`LockMismatch`).
*   Qualquer outra falha imprevista → `500`.

**Forma curta (exemplo de enum)**

```rust
#[derive(thiserror::Error, Debug)]
pub enum ApiError {
  #[error("invalid input: {0}")]
  InvalidInput(String),     // -> 400
  #[error("lock mismatch: {0}")]
  LockMismatch(String),     // -> 409
  #[error("internal error")]
  Internal(#[from] anyhow::Error), // -> 500
}
// IntoResponse mapeia status + JSON {"error":"...", "req_id":"..."}
```

* * *

5) Opções de execução (CLI/ENV)
===============================

*   `--port <u16>` / `PORT` (default: 8787)
*   `--addr <ip>` (default: 0.0.0.0)
*   `--manifest <path>` (default: embed do produto)
*   `--lock <path>` ou embutido no bin
*   `--timeout-ms <u64>` (default: 15000)
*   `--max-bytes <usize>` (default: 1\_048\_576)
*   `--cors-allow-origins <csv>`
*   `--resolver-base <url>` (p/ montar `url_rica`)
*   Logs estruturados: `RUST_LOG=info,server=debug` + **tracing** JSON (`--log-format json`)
*   **Graceful shutdown**: `SIGINT/SIGTERM` com `with_graceful_shutdown`

* * *

6) Esqueleto (Axum 0.7 + Tower + Tracing) — **shape**
=====================================================

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // logs estruturados
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    let cfg = ServerCfg::from_env()?; // porta, timeout, cors, max_bytes...

    // pré-carrega manifest normalizado + lock + registry
    let (manifest, lock, cap_registry) = bootstrap_product(&cfg).await?;

    // middlewares
    use tower::{ServiceBuilder, timeout::TimeoutLayer};
    let layers = ServiceBuilder::new()
        .layer(TimeoutLayer::new(cfg.timeout))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(cfg.max_bytes))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::cors::CorsLayer::permissive()) // ajustar cfg
        .layer(tower_http::compression::CompressionLayer::new())
        .into_inner();

    // rotas
    let app = axum::Router::new()
        .route("/evaluate", axum::routing::post({
            let manifest = manifest.clone();
            let lock = lock.clone();
            let reg = cap_registry.clone();
            move |req| handle_evaluate(req, manifest.clone(), lock.clone(), reg.clone(), cfg.clone())
        }))
        .route("/health", axum::routing::get(handle_health))
        .route("/version", axum::routing::get(handle_version))
        .layer(layers);

    // graceful shutdown (ctrl+c ou SIGTERM em prod)
    let listener = tokio::net::TcpListener::bind((cfg.addr, cfg.port)).await?;
    tracing::info!(%cfg.port, "server starting");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
```

**`handle_evaluate` (resumo do fluxo)**

```rust
async fn handle_evaluate(
    axum::extract::Json(input): axum::extract::Json<serde_json::Value>,
    manifest: RunnerManifest,
    lock: ModulesLock,
    reg: CapRegistry,
    cfg: ServerCfg,
) -> Result<axum::Json<HttpResult>, ApiError> {
    // 1) JSON -> NRF
    let env = ubl_json_view::from_json(&input)
        .map_err(|e| ApiError::InvalidInput(format!("json->nrf: {e}")))?;

    // 2) validar lock (major/* + CID)
    validate_lock(&manifest, &reg, &lock).map_err(|e| ApiError::LockMismatch(e.to_string()))?;

    // 3) executar runner (async)
    let out = run_pipeline_async(&manifest, env, &reg, &cfg).await
        .map_err(ApiError::Internal)?;

    // 4) mapear HttpResult
    Ok(axum::Json(out))
}
```

* * *

7) Aceite: **curl** simples retorna `receipt_cid + url_rica`
============================================================

**Boot**

```bash
./target/release/<produto-bin> --port 8787 --resolver-base https://resolver.local/r
```

**Saúde**

```bash
curl -s http://127.0.0.1:8787/health
# {"status":"ok"}
```

**Versão**

```bash
curl -s http://127.0.0.1:8787/version
# {"version":"1.0.0","git_sha":"47dff9b","build_ts":"2026-02-12T11:23:00Z"}
```

**Avaliar**

```bash
curl -s -X POST http://127.0.0.1:8787/evaluate \
  -H "Content-Type: application/json" \
  -d '{"data":"hello"}' | jq
# {
#   "product": "tdln-policy",
#   "ok": true,
#   "verdict": "Allow",
#   "receipt_cid": "b3:6780c2ca...",
#   "url_rica": "https://resolver.local/r/b3:6780c2ca...",
#   "stopped_at": null,
#   "receipt_chain": ["b3:...", "..."],
#   "metrics": [...],
#   "artifacts": 2
# }
```

> **Passa no aceite** se `/evaluate` responde `200` com **`receipt_cid`** e **`url_rica`** preenchidos. Se a policy negar em cenário de teste, ainda assim deve retornar `200` com `verdict:"Deny"`.

* * *

8) Extras úteis (pós-MVP)
=========================

*   **/explain/{cid}**: re-render do status page a partir do RICA (read-only)
*   **/events**: stream SSE com métricas (dev-mode)
*   **TLS**: `--tls-cert --tls-key` (rustls)
*   **Auth**: HMAC/JWT opcional no `POST /evaluate`
*   **Audit**: log JSON com `receipt_cid`, `verdict`, `latency_ms`

* * *


Objetivo: produzir container pronto para deploy.
Entregáveis:

--container: Dockerfile multi-stage, usuário não-root, variável para porta, healthcheck.

Manifesto de exemplo para K8s/Compose.
Aceite: docker run + curl funcionam; imagem <tamanho-alvo> razoável.


* * *

Objetivo
========

Entregar uma **imagem OCI leve, segura e reprodutível** do **bin gerado pelo “Gerador de Produtos”**, pronta para `docker run`, Kubernetes e Docker Compose.

> Premissas:
> 
> *   O binário chama-se `product` (gerado por `ubl product from-manifest`).
> *   Servidor HTTP interno lê `PORT` (default 8787) e expõe `/evaluate`, `/health`, `/version`.
> *   Build com **feature `runner-real`** quando quiser pipeline real.
>     

* * *

1) Dockerfile (multi-stage, não-root, healthcheck)
==================================================

### Variante A — **Debian slim** (mais simples/compatível; ~55–80 MB)

```dockerfile
# syntax=docker/dockerfile:1.7
ARG RUST_VERSION=1.76
ARG APP_NAME=product

########################
# Stage 1: builder
########################
FROM rust:${RUST_VERSION}-slim AS builder

# deps para build (openssl/ca-certificates se o bin faz TLS outbound)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates build-essential clang && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

# cache inteligente das dependências
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY modules ./modules
COPY tools ./tools
COPY manifests ./manifests
# se o gerador emitir o binário em ./generated/<product>:
COPY generated ./generated

# build do bin do produto (ajuste o path se necessário)
# Exemplo: o gerador produz um crate binário em ./generated/product-bin
WORKDIR /build/generated/product-bin
# Feature gate opcional: --features runner-real
RUN cargo build --release --features runner-real

########################
# Stage 2: runtime
########################
FROM debian:stable-slim AS runtime

# (opcional) camada de certificados para HTTPs outbound
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates tzdata curl && \
    rm -rf /var/lib/apt/lists/*

# usuário não-root (UID/GID fixos para K8s/OPA politicas)
RUN groupadd -g 10001 app && useradd -u 10001 -g 10001 -s /sbin/nologin -M app

# diretórios
WORKDIR /app
RUN mkdir -p /app/bin /app/data /app/config && chown -R app:app /app

# copia o binário
COPY --from=builder /build/target/release/${APP_NAME} /app/bin/${APP_NAME}

# porta e flags padrão
ENV PORT=8787 \
    RUST_LOG=info \
    RESOLVER_BASE=https://resolver.local/r \
    TIMEOUT_MS=15000 \
    MAX_BYTES=1048576

# segurança mínima
USER app

# healthcheck (ajuste o path se preferir usar /health)
HEALTHCHECK --interval=30s --timeout=2s --start-period=10s --retries=3 \
  CMD curl -fsS "http://127.0.0.1:${PORT}/health" || exit 1

EXPOSE 8787
ENTRYPOINT ["/app/bin/product"]
```

### Variante B — **alpine + MUSL estático** (menor; ~15–25 MB)

> Requer toolchain MUSL e, às vezes, ajustes de OpenSSL. Use se o bin não precisa de `libssl` dinâmica.

```dockerfile
# syntax=docker/dockerfile:1.7
ARG RUST_VERSION=1.76
ARG TARGET=x86_64-unknown-linux-musl
ARG APP_NAME=product

FROM rust:${RUST_VERSION}-alpine AS builder
RUN apk add --no-cache musl-dev build-base openssl-libs-static openssl-dev pkgconfig
RUN rustup target add ${TARGET}
WORKDIR /build
COPY . .
# importante: linkagem estática
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN cargo build --release --target ${TARGET} --features runner-real

FROM alpine:3.20 AS runtime
RUN addgroup -g 10001 app && adduser -D -u 10001 -G app app
WORKDIR /app
COPY --from=builder /build/target/${TARGET}/release/${APP_NAME} /app/${APP_NAME}
ENV PORT=8787 RUST_LOG=info RESOLVER_BASE=https://resolver.local/r TIMEOUT_MS=15000 MAX_BYTES=1048576
USER app
HEALTHCHECK --interval=30s --timeout=2s --start-period=10s --retries=3 \
  CMD wget -qO- "http://127.0.0.1:${PORT}/health" >/dev/null 2>&1 || exit 1
EXPOSE 8787
ENTRYPOINT ["/app/product"]
```

**Boas práticas incluídas**

*   🚫 **não-root** (`USER app`)
*   🔒 **packages mínimos** e `rm -rf /var/lib/apt/lists/*`
*   🌡️ **HEALTHCHECK** padrão
*   🌍 **ENV PORT** (server já lê `PORT`)
*   🪵 **RUST\_LOG** default estruturado (ajuste no bin com `tracing`)

* * *

2) Manifesto de exemplo — **Kubernetes (Deployment + Service)**
===============================================================

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: product
  labels: { app: product }
spec:
  replicas: 1
  selector:
    matchLabels: { app: product }
  template:
    metadata:
      labels: { app: product }
    spec:
      securityContext:
        runAsUser: 10001
        runAsGroup: 10001
        runAsNonRoot: true
      containers:
        - name: product
          image: ghcr.io/ORG/PRODUCT:1.0.0
          imagePullPolicy: IfNotPresent
          env:
            - name: PORT
              value: "8787"
            - name: RUST_LOG
              value: "info,server=debug"
            - name: RESOLVER_BASE
              value: "https://resolver.local/r"
            - name: TIMEOUT_MS
              value: "15000"
            - name: MAX_BYTES
              value: "1048576"
            # segredos de providers (ex.: OpenAI/Ollama) via Secret:
            # - name: OPENAI_API_KEY
            #   valueFrom: { secretKeyRef: { name: openai, key: apiKey } }
          ports:
            - name: http
              containerPort: 8787
          readinessProbe:
            httpGet: { path: /health, port: http }
            periodSeconds: 10
          livenessProbe:
            httpGet: { path: /health, port: http }
            initialDelaySeconds: 10
            periodSeconds: 30
          resources:
            requests: { cpu: "100m", memory: "128Mi" }
            limits:   { cpu: "500m", memory: "512Mi" }
---
apiVersion: v1
kind: Service
metadata:
  name: product
spec:
  selector: { app: product }
  ports:
    - name: http
      port: 80
      targetPort: 8787
  type: ClusterIP
```

* * *

3) Manifesto de exemplo — **Docker Compose**
============================================

```yaml
version: "3.9"
services:
  product:
    image: ghcr.io/ORG/PRODUCT:1.0.0
    container_name: product
    restart: unless-stopped
    environment:
      PORT: "8787"
      RUST_LOG: "info,server=debug"
      RESOLVER_BASE: "https://resolver.local/r"
      TIMEOUT_MS: "15000"
      MAX_BYTES: "1048576"
      # OPENAI_API_KEY: "${OPENAI_API_KEY}"
    ports:
      - "8787:8787"
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://127.0.0.1:8787/health >/dev/null 2>&1 || exit 1"]
      interval: 30s
      timeout: 2s
      retries: 3
      start_period: 10s
```

* * *

4) Build & Push (single e multi-arch)
=====================================

**Single-arch (x86\_64)**

```bash
docker build -t ghcr.io/ORG/PRODUCT:1.0.0 -f Dockerfile .
docker run --rm -e PORT=8787 -p 8787:8787 ghcr.io/ORG/PRODUCT:1.0.0
```

**Multi-arch (linux/amd64 + linux/arm64)**

```bash
docker buildx create --use --name builder || true
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t ghcr.io/ORG/PRODUCT:1.0.0 \
  -f Dockerfile \
  --push .
```

> Dica: se o gerador produz múltiplos bins (um por produto), reutilize o mesmo Dockerfile com `--build-arg APP_NAME=<bin>`.

* * *

5) Checklist de **Aceite**
==========================

*   ✅ `docker run` sobe na porta 8787 (ou `PORT` customizado).
*   ✅ `curl http://127.0.0.1:8787/health` → `{"status":"ok"}`
*   ✅ `curl -s -X POST :8787/evaluate -H 'Content-Type: application/json' -d '{"data":"hello"}'`  
    → resposta `200` com `receipt_cid` e `url_rica`.
*   ✅ Imagem **< 80 MB** (Debian slim) **ou < 25 MB** (Alpine/MUSL), conforme variante.
*   ✅ Rodando como **não-root**.
*   ✅ HEALTHCHECK passa.

* * *

6) Endurecimento (opcional, recomendado) 🔐
===========================================

*   **OCI Labels** (proveniência/metadata)
    ```dockerfile
    LABEL org.opencontainers.image.title="AI-NRF1 Product"
    LABEL org.opencontainers.image.description="Generated product binary"
    LABEL org.opencontainers.image.revision="${GIT_SHA}"
    LABEL org.opencontainers.image.version="1.0.0"
    ```
*   **SBOM**: `syft ghcr.io/ORG/PRODUCT:1.0.0 -o spdx-json=sbom.spdx.json`
*   **Assinatura**: `cosign sign ghcr.io/ORG/PRODUCT:1.0.0`
*   **Proveniência (SLSA/GitHub OIDC)**: ative `attest` no pipeline
*   **Root FS imutável** no K8s:
    ```yaml
    securityContext:
      readOnlyRootFilesystem: true
      allowPrivilegeEscalation: false
      capabilities: { drop: ["ALL"] }
    ```
*   **Distroless** no runtime (alternativa à Debian/Alpine) se o bin não precisa de shell/utilitários.

* * *

7) Dicas de tamanho e cache 💡
==============================

*   Ordene `COPY` para **maximizar cache** das dependências (primeiro `Cargo.*`).
*   Use **alpine+musl** quando possível (bin estático → imagem mínima).
*   Remova ferramentas de build no runtime; **copie só o bin**.
*   Evite `ENV` que quebrem cache entre builds se não for necessário.

* * *

8) Variáveis úteis (resumo)
===========================

| VAR | Default | Uso |
| --- | --- | --- |
| `PORT` | `8787` | Porta de escuta HTTP |
| `RUST_LOG` | `info` | Nível de logs (tracing) |
| `RESOLVER_BASE` | `https://resolver.local/r` | Monta `url_rica` |
| `TIMEOUT_MS` | `15000` | Timeout por request |
| `MAX_BYTES` | `1048576` | Limite de payload |
| `OPENAI_API_KEY`/`OLLAMA_BASE_URL` | — | Providers LLM, quando habilitado |

* * *


Objetivo: garantir qualidade e não-regressão.
Entregáveis:

Testes unit (schema/aliases/lock), integração (smokes dos 4 produtos), snapshots do --explain.

Jobs CI (matriz features=[stub, runner-real]), gating de providers por env.

Critérios de aceite objetivos por cenário Allow/Deny/Ask.
Aceite: pipeline verde com artefatos JSON de execução.


* * *

Objetivo
========

Garantir **qualidade, não-regressão e reproduzibilidade** do Gerador e dos 4 produtos canônicos:

*   `tdln-policy`
*   `tdln-runtime`
*   `llm-engine`
*   `llm-smart`

Resultados sempre materializados em **artefatos JSON**.

* * *

1) Testes Unitários (schema/aliases/lock)
=========================================

### 1.1 Schema do manifesto e aliases

*   **Crate**: `crates/tdln` (ou onde está o parser do manifesto).
*   **Cobertura**:
    *   `manifest::load()` valida contra JSON-Schema.
    *   Tradução de **aliases → kinds reais** (`policy.light → cap-policy` etc.).
    *   Regras de **version**: `major`/`*` e fallback para _lock_.
*   **Ferramentas**: `serde_json`, `jsonschema`, `insta` (snapshots).

Exemplo (Rust):

```rust
#[test]
fn manifest_schema_and_aliases() {
    let raw = include_str!("../../fixtures/manifests/llm-engine.yaml");
    let m = tdln::Manifest::from_yaml(raw).unwrap();
    assert!(m.validate_schema().is_ok());
    let expanded = m.expand_aliases(&alias_map());
    insta::assert_json_snapshot!("llm_engine_expanded", &expanded);
}
```

### 1.2 Lockfile e pin por CID

*   **Crate**: `crates/tdln-lock`.
*   **Cobertura**:
    *   `modules.lock.json` → estrutura, _pin_ por BLAKE3, _diff_ semântico.
    *   Erro claro quando `manifest.version` não casa com lock (major mismatch).

```rust
#[test]
fn lock_pin_by_cid() {
    let lock = tdln_lock::Lock::from_str(include_str!("../../fixtures/locks/modules.lock.json")).unwrap();
    assert!(lock.resolve("cap-llm").unwrap().cid.starts_with("b3:"));
}
```

* * *

2) Testes de Integração (smokes dos 4 produtos)
===============================================

### 2.1 Runner “stub” (rápido, determinístico)

*   **Crate**: `tools/ubl-cli` (testes de binário com `assert_cmd` / `snapbox`).
*   **Cenários**:
    *   `tdln-policy --var data=hello` → **Allow** (receipts > 0, `stopped_at: null`).
    *   `tdln-policy` sem `data` → **Deny** (`stopped_at == step-1-cap-policy`).
    *   `tdln-runtime` com `userCode` → **Allow** e métrica `runtime.request == 1`.
    *   `llm-engine`/`llm-smart` → **Allow** com `prompt_len` > 0.
*   **Artefatos**: gravar stdout dos comandos em `target/artifacts/smoke/<product>/<ts>.json`.

Exemplo:

```rust
#[test]
fn smoke_tdln_policy_allow() {
    let mut cmd = assert_cmd::Command::cargo_bin("ubl").unwrap();
    let out = cmd.args(["tdln","policy","--var","data=hello"])
        .assert()
        .success()
        .get_output()
        .stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["result"]["verdict"], "Allow");
    assert!(v["result"]["receipt_chain"].as_array().unwrap().len() >= 2);
}
```

### 2.2 Runner “real” (feature `runner-real`)

*   **Gating**: roda só se `RUNNER_REAL=1`.
*   **Providers**:
    *   `OPENAI_API_KEY` ausente → **pula** casos que dependem de Engine.
    *   `OLLAMA_BASE_URL` ausente → **pula** Smart.
*   **Critérios** (ver Seção 5) e geração de artefatos JSON.

* * *

3) Snapshots do `--explain` (DAG expandida)
===========================================

*   **Comando**: `ubl tdln explain --manifest <...>`
*   **Onde**: `tests/snapshots/explain_*`
*   **Proteção**: normalizar campos “voláteis” (timestamps, cids de build) antes do snapshot.
*   **Objetivo**: _diff legível_ quando mudar encadeamento (ex.: bundles → pipeline).

Exemplo:

```rust
#[test]
fn explain_llm_engine_snapshot() {
    let json = run_and_capture(&["tdln","explain","--manifest","manifests/llm/engine.yaml"]);
    let normalized = normalize_explain(json);
    insta::assert_yaml_snapshot!("explain_llm_engine", normalized);
}
```

* * *

4) QA Manual leve (fornecido como script)
=========================================

*   **Script**: `scripts/qa_smoke.sh`
*   **Checks**:
    *   `./product --version` imprime versão, `git_sha`, `build_ts`.
    *   `curl :8787/health` → `{"status":"ok"}`.
    *   `curl :8787/evaluate` com payload mínimo → `200`, `receipt_cid`, `url_rica`.
*   Artefatos em `artifacts/qa/`.

* * *

5) Critérios de Aceite (Allow/Deny/Ask)
=======================================

| Cenário | Input mínimo | Expectativa | Checks objetivos |
| --- | --- | --- | --- |
| **Allow** (policy) | `--var data=hello` | Pipeline completo | `verdict=="Allow"`, `stopped_at==null`, `receipt_chain.len>=3` |
| **Deny** (policy) | sem `data` | Para em policy | `verdict=="Deny"`, `stopped_at=="step-1-cap-policy"` |
| **Ask** (policy) | regra `ASK_IF` simulada (ex.: `THRESHOLD_RANGE` sem claridade) | Retorna `Ask` | `verdict=="Ask"`, campo `ask.reasons[].code` presente |
| **Runtime Allow** | `--var sourceData=x --var userCode=print` | Executa runtime | métrica `runtime.request==1`, `verdict=="Allow"` |
| **Engine Allow** | `--var prompt=hi` + `OPENAI_API_KEY` | LLM executa | `prompt_len>0`, `max_tokens>0`, `verdict=="Allow"` |
| **Smart Allow** | `--var prompt=hi` + `OLLAMA_BASE_URL` | LLM local | idem Engine |

> **Saída obrigatória**: sempre JSON com `receipt_cid` e `url_rica`.

* * *

6) CI: Matriz, Gating e Artefatos
=================================

### 6.1 GitHub Actions (exemplo)

Arquivo: `.github/workflows/ci.yml`

```yaml
name: CI
on:
  push:
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  build-and-test:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        feature: [stub, runner-real]
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Build (feature)
        run: |
          if [ "${{ matrix.feature }}" = "runner-real" ]; then
            cargo build --workspace --features runner-real
          else
            cargo build --workspace
          fi

      - name: Unit tests
        run: |
          if [ "${{ matrix.feature }}" = "runner-real" ]; then
            cargo test -p tdln -p tdln-lock -p ubl-cli --features runner-real
          else
            cargo test -p tdln -p tdln-lock -p ubl-cli
          fi

      - name: Smoke (stub)
        if: matrix.feature == 'stub'
        run: |
          mkdir -p artifacts/smoke
          ./target/debug/ubl tdln policy --var data=hello | tee artifacts/smoke/tdln-policy-allow.json
          ./target/debug/ubl tdln policy | tee artifacts/smoke/tdln-policy-deny.json
          ./target/debug/ubl tdln runtime --var sourceData=x --var userCode=print | tee artifacts/smoke/tdln-runtime-allow.json

      - name: Smoke (runner-real, gated)
        if: matrix.feature == 'runner-real'
        env:
          RUNNER_REAL: "1"
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
          OLLAMA_BASE_URL: ${{ vars.OLLAMA_BASE_URL }}
        run: |
          mkdir -p artifacts/smoke-real
          ./target/debug/ubl tdln policy --var data=hello | tee artifacts/smoke-real/policy-allow.json
          ./target/debug/ubl tdln runtime --var sourceData=x --var userCode=print | tee artifacts/smoke-real/runtime-allow.json
          if [ -n "${OPENAI_API_KEY}" ]; then
            ./target/debug/ubl llm engine --var prompt=hello | tee artifacts/smoke-real/engine-allow.json || true
          fi
          if [ -n "${OLLAMA_BASE_URL}" ]; then
            ./target/debug/ubl llm smart --var prompt=hello | tee artifacts/smoke-real/smart-allow.json || true
          fi

      - name: Explain snapshots
        run: |
          ./target/debug/ubl tdln explain --manifest manifests/llm/engine.yaml > artifacts/explain/llm-engine.yaml || true
          ./target/debug/ubl tdln explain --manifest manifests/tdln/policy.yaml > artifacts/explain/tdln-policy.yaml || true

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: ci-artifacts-${{ matrix.feature }}
          path: artifacts
```

**Observações**

*   Matriz `feature=[stub, runner-real]`.
*   **Gating por env**: só roda Engine/Smart se as variáveis existirem.
*   Publica **artefatos** (`artifacts/**`) para inspeção pós-build.

* * *

7) Convenções de Artefatos
==========================

```
artifacts/
  smoke/                     # stub
    tdln-policy-allow.json
    tdln-policy-deny.json
    tdln-runtime-allow.json
  smoke-real/                # runner-real
    engine-allow.json        # se OPENAI_API_KEY presente
    smart-allow.json         # se OLLAMA_BASE_URL presente
  explain/
    llm-engine.yaml
    tdln-policy.yaml
```

* * *

8) Regressão controlada (insta + normalização)
==============================================

Criar utilitário `tests/helpers/snap.rs` para:

*   **normalizar** campos voláteis (`ts`, `build_ts`, `cid`, `duration_ms` → placeholders).
*   snapshot estável.

```rust
pub fn normalize(mut v: serde_json::Value) -> serde_json::Value {
    fn zero(k: &str, v: &mut serde_json::Value) {
        if let Some(x) = v.pointer_mut(k) { *x = serde_json::json!("__REDUCTED__"); }
    }
    zero("/result/receipt_cid", &mut v);
    zero("/result/url_rica", &mut v);
    v
}
```

* * *

9) Cobertura e Saúde do Projeto
===============================

*   **Linters**: `cargo fmt -- --check`, `cargo clippy -- -D warnings`
*   **Segurança**: `cargo deny check` (licenças/vulns)
*   **Fuzz (opcional)**: alvos no `nrf-core` (parser/encode)
*   **nextest (opcional)**: paralelismo e relatórios estáveis

* * *

10) Aceite da Parte 8 ✅
=======================

*   CI executa **matriz** `stub` e `runner-real`.
*   Todos os **unit** passam (schema, aliases, lock).
*   **Smokes** dos 4 produtos:
    *   `tdln-policy` → **Allow** e **Deny** provados.
    *   `tdln-runtime` → **Allow** provado.
    *   `llm-engine` / `llm-smart` → **Allow** quando `OPENAI_API_KEY`/`OLLAMA_BASE_URL` presentes; senão **marcados como skipped**.
*   **Explain snapshots** gerados e versionados.
*   **Artefatos JSON** anexados no job.
*   Linters verdes.

* * *


Objetivo: operacionalizar em produção.
Entregáveis:

Logs estruturados (campos padronizados), métricas (por step/latência), IDs de correlação (cid/hop).

Controles de segurança (limites, deny-lists, validação de entrada, headers de segurança).

Política de segredos/keys (nenhum segredo em recibos/artifacts).
Aceite: checklist de segurança + exemplo de dashboard.


* * *

Objetivo
========

Operacionalizar o binário gerado com **visibilidade ponta-a-ponta**, **controles preventivos** e **política de segredos** que respeita os recibos/capsules (nada sensível vaza).

* * *

1) Logs estruturados (JSON) + correlação
========================================

1.1 Esquema canônico
--------------------

Cada linha é um JSON com chaves padrão:

*   `ts` (RFC3339)
*   `level` (`INFO|WARN|ERROR`)
*   `message`
*   `service` (ex.: `product-bin`)
*   `product` (ex.: `tdln-policy`)
*   `route` (`/evaluate`, `/health`, etc.)
*   `method`, `status`, `latency_ms`
*   **Correlação**: `request_id`, `receipt_cid`, `hop` (índice do pipeline), `step_kind`, `step_id`
*   **Tenant** (se houver multitenancy): `tenant_id`
*   **CID de entrada/artefato**: `env_cid`, `artifact_cid` (quando aplicável)
*   **network**: `remote_ip`, `user_agent` (anonimizados/hasheados, se necessário compliance)

Exemplo (linha única):

```json
{
  "ts":"2026-02-12T11:22:33.421Z",
  "level":"INFO",
  "service":"product-bin",
  "product":"tdln-policy",
  "route":"/evaluate",
  "method":"POST",
  "status":200,
  "latency_ms":18,
  "request_id":"rq_6H2c9uWQ",
  "receipt_cid":"b3:6780c2...",
  "hop":2,
  "step_id":"step-1-cap-policy",
  "step_kind":"cap-policy",
  "tenant_id":"lab512",
  "remote_ip":"hash:4f0b3e...",
  "user_agent":"hash:a1c9...",
  "message":"pipeline.step.completed"
}
```

1.2 Implementação (Rust)
------------------------

*   `tracing` + `tracing-subscriber` (JSON), camada para **enriquecer spans** com `request_id`, `receipt_cid`, `product`.
*   Middleware HTTP injeta `request_id` (ou usa `X-Request-Id` válido).
*   No executor do pipeline, abrir um **span** por step (`step_id`, `step_kind`) e marcar métricas.

* * *

2) Métricas (Prometheus/OpenTelemetry)
======================================

2.1 Tabela de métricas
----------------------

| Métrica | Tipo | Labels | Descrição |
| --- | --- | --- | --- |
| `product_requests_total` | counter | `product`, `route`, `method`, `status` | Total de requisições |
| `product_request_duration_ms` | histogram | `product`, `route`, `method`, `status` | Latência HTTP |
| `pipeline_steps_total` | counter | `product`, `step_kind`, `result` (\`ok | error |
| `pipeline_step_duration_ms` | histogram | `product`, `step_kind` | Latência por step |
| `pipeline_receipts_total` | counter | `product` | Receipts gerados (hop count) |
| `policy_verdict_total` | counter | `product`, `verdict` (\`Allow | Deny |
| `llm_tokens_total` | counter | `product`, `engine` (\`engine | smart\`) |
| `llm_requests_total` | counter | `product`, `engine`, `provider` | Chamadas LLM |
| `runtime_exec_total` | counter | `product`, `runtime` (`wasm`) | Execuções no Certified Runtime |
| `runtime_exec_errors_total` | counter | `product`, `error_kind` | Falhas no runtime |
| `transport_publish_total` | counter | `product`, `target` (\`s3 | webhook |
| `security_rejections_total` | counter | `product`, `reason` | Bloqueios por segurança (payload-limit, denylist, schema, etc.) |

> Endpoint `/metrics` exposto (Prometheus), com **scrape** padrão.

2.2 Buckets sugeridos
---------------------

*   Requests: `5, 10, 25, 50, 100, 250, 500, 1000` ms
*   Steps: `1, 2, 5, 10, 25, 50, 100, 250` ms
*   Tokens (counter simples; custo calculado off-path)

* * *

3) Tracing distribuído (opcional, recomendado)
==============================================

*   OpenTelemetry (`otlp`) → Jaeger/Tempo.
*   Um **trace por request**, **span por step**.
*   Propagar `traceparent` nos headers de saída (útil quando chamar webhooks).
*   Amostragem: 1% default; 100% para `ERROR` e `verdict=Deny/Ask`.

* * *

4) Controles de segurança
=========================

4.1 Entrada e limites
---------------------

*   **Payload limit**: ex. `2 MiB` (configurável).
*   **Timeout end-to-end**: ex. `3s–10s` (por produto/rota).
*   **Schema validation** do request (JSON → `json_view` → NRF); erro `422` com motivo.
*   **Deny-lists**:
    *   Campos proibidos/overrides (ex. `verdict`, `step_override`).
    *   Origem IP (lista bloqueada) — _se aplicável_.
*   **WAF/Rate-limit** (na borda/reverso): `N req/s` por IP/tenant.

4.2 WASM / Certified Runtime
----------------------------

*   **Sandbox**:
    *   Desabilitar `wasi:filesystem` e `wasi:random` não-determinista (ou prover RNG determinístico).
    *   Tempo de CPU: `fuel`/instr quota.
    *   Memória: `limit` (ex. 64–256MB por execução).
    *   **No network** dentro do WASM (capabilities explícitas).
*   **Policy de módulos**: só módulos **pinned por CID** via `modules.lock.json`.
    *   Se `kind@version` não casar com o `cid` do lock → `HTTP 409` e log `WARN`.

4.3 Cabeçalhos de segurança (HTTP)
----------------------------------

*   `Strict-Transport-Security: max-age=31536000; includeSubDomains`
*   `X-Content-Type-Options: nosniff`
*   `Content-Security-Policy: default-src 'none'` (para `/r/{cid}` página HTML, permitir apenas o necessário)
*   `Referrer-Policy: no-referrer`
*   `Permissions-Policy: geolocation=(), microphone=(), camera=()`
*   `Cross-Origin-Opener-Policy: same-origin` (se tiver UI embutida)
*   **CORS**: apenas domínios permitidos; `Vary: Origin`.

4.4 Erros e redaction
---------------------

*   Nunca retornar stack trace em produção.
*   Redigir (`******`) valores compatíveis com padrões de segredo/PII (regex simples: keys `api_key|secret|token|authorization`, e valores que se pareçam com chaves).
*   Logar **códigos** e **CIDs**, não payloads brutos (exceto em nível DEBUG controlado).

* * *

5) Política de segredos/keys 🔑
===============================

*   **Origem**: somente via ambiente/secret manager (ex.: `OPENAI_API_KEY`, `OLLAMA_BASE_URL`).
*   **Nunca** serializar segredos em:
    *   recibos (`receipt`/`artifacts`)
    *   métricas (labels ou valores)
    *   logs (message/fields)
*   **Mask**:
    *   Se `Authorization` header presente, **não** logar; se indispensável, logar `sha256:xxxx`.
*   **Rotação**:
    *   Recarregar chaves por SIGHUP ou ler a cada N minutos (sem reiniciar).
*   **Permissões**:
    *   Arquivo/volume de secrets com `0600`; usuário não-root no container.
*   **CI**:
    *   Jobs runner-real só sob `secrets` presentes. Artefatos **não** contêm payloads/headers crus.

* * *

6) Exemplo de dashboard (Grafana)
=================================

**Folder: Products / Gerador**

1.  **SLO — HTTP Latency**
    *   `product_request_duration_ms` (p95, p99) por `product` e `route`
2.  **Throughput**
    *   `product_requests_total` (rate 1m) por `status`
3.  **Policy Verdicts**
    *   `policy_verdict_total` por `verdict` (stacked)
4.  **Pipeline Steps**
    *   `pipeline_steps_total{result="error"}` (rate) por `step_kind`
    *   `pipeline_step_duration_ms` p95 por `step_kind`
5.  **LLM Custo/Volume**
    *   `llm_tokens_total` (rate) por `engine` + painel calculado `USD = tokens_in*price_in + tokens_out*price_out`
6.  **Runtime Health**
    *   `runtime_exec_total` vs `runtime_exec_errors_total`
7.  **Security**
    *   `security_rejections_total` por `reason`
8.  **Receipts**
    *   `pipeline_receipts_total` (rate) + _singlestat_ “avg hops por request”

* * *

7) Checklist de aceite ✅
========================

*   **Logs estruturados** ativados (JSON), com `request_id`, `receipt_cid`, `step_id/step_kind`.
*   **/metrics** expõe todas as métricas listadas; Prometheus scrape ok.
*   **Tracing** opcional habilitável por flag/env; spans por step.
*   **Limits**: payload, timeout e rate-limit configuráveis; negações contabilizadas em `security_rejections_total`.
*   **Validação** de entrada com schema e códigos corretos (`400/401/403/422/429/500`).
*   **Headers de segurança** presentes nas rotas públicas.
*   **Segredos** só por env/secret manager; masking em logs; nada sensível em recibos/artifacts.
*   **Dashboard** importável (JSON) com painéis descritos acima.

* * *

8) Trechos prontos (colar e usar)
=================================

8.1 Boot de logging/tracing (Rust)
----------------------------------

```rust
use tracing_subscriber::{fmt, EnvFilter};
pub fn init_observability() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,hyper=warn,tower=warn"));
    fmt()
      .with_env_filter(filter)
      .json()
      .with_current_span(false)
      .flatten_event(true)
      .init();
}
```

8.2 Middleware de request\_id (Axum/Tower)
------------------------------------------

```rust
use axum::{http::HeaderMap, Extension};
use nanoid::nanoid;
pub async fn with_request_id(headers: HeaderMap) -> String {
    if let Some(id) = headers.get("x-request-id").and_then(|v| v.to_str().ok()) {
        id.to_string()
    } else { format!("rq_{}", nanoid!(10)) }
}
```

8.3 Prometheus (axum-extra)
---------------------------

```rust
use axum_prometheus::PrometheusMetricLayer;
let (prom_layer, prom_handle) = PrometheusMetricLayer::pair();
let app = Router::new()
  .route("/metrics", get(|| async move { prom_handle.render() }))
  .layer(prom_layer);
```

8.4 Headers de segurança (Tower)
--------------------------------

```rust
use tower_http::set_header::SetResponseHeaderLayer;
use http::header::{STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS, CONTENT_SECURITY_POLICY};
let app = app
  .layer(SetResponseHeaderLayer::overriding(STRICT_TRANSPORT_SECURITY, "max-age=31536000; includeSubDomains".parse().unwrap()))
  .layer(SetResponseHeaderLayer::overriding(X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap()))
  .layer(SetResponseHeaderLayer::overriding(CONTENT_SECURITY_POLICY, "default-src 'none'".parse().unwrap()));
```

* * *

9) Operação & Runbook (resumo)
==============================

*   **Alerta** quando:
    *   `policy_verdict_total{verdict="Deny"}` ↑ 3× baseline
    *   `pipeline_steps_total{result="error"}` > 0.5/s por 5 min
    *   `product_request_duration_ms{status="200"}` p99 > 500ms
*   **Procedimento**:
    1.  Consultar painel “Security” e “Steps Errors”.
    2.  Pegar `request_id` do alerta → buscar logs correlacionados (Kibana/Loki).
    3.  Conferir `receipt_cid` e `hop` no status page `/r/{cid}`.
    4.  Se LLM: checar `llm_requests_total` e limites de provider.

* * *


Objetivo: colar frontend ao gerador e ao registry.
Entregáveis:

lib/api.ts (cliente tipado), variáveis de ambiente, troca de mocks.

Wire das telas executions e console/r/[cid] para endpoints reais.
Aceite: fluxo do console executa, mostra SIRP/timeline, e abre o recibo via /r/{cid}.


Abaixo está a **Parte 10 — Integração com a UI (Console/Resolver)** com passos práticos, arquivos, tipos e pontos de atenção para você “trocar os mocks” pelos endpoints reais do **binário de produto** e do **registry**.

* * *

Visão Geral
===========

*   **Frontend**: Next.js (App Router), TypeScript, shadcn/ui. Hoje usa `lib/mock-data.ts`.
*   **Backends reais**:
    *   **Produto** (bin gerado pelo `ubl product from-manifest`): expõe pelo menos `POST /evaluate`, `GET /health`, `GET /version` (e opcionalmente `/metrics`).
    *   **Registry/Resolver**: `GET /r/{cid}` com content negotiation:
        *   `Accept: application/ai-nrf1` → bytes NRF/JSON (dados canônicos do recibo).
        *   `Accept: text/html` → página HTML (status page rica).

**Objetivo**:

1.  Console **/console/executions** chama **/evaluate** e lista execuções (cache local + opcional proxy /api).
2.  Detalhe do recibo **/console/r/\[cid\]** consome **/r/{cid}** e desenha **SIRP** + “timeline”.

* * *

10.1. Variáveis de Ambiente
===========================

Crie `.env.local` na raiz do frontend:

```env
NEXT_PUBLIC_PRODUCT_BASE_URL=http://127.0.0.1:8080
NEXT_PUBLIC_REGISTRY_BASE_URL=http://127.0.0.1:8791
# Opcional: chave de leitura pública, se existir camada de API-gateway na frente
NEXT_PUBLIC_CONSOLE_ORIGIN=http://localhost:3000
```

> **CORS no servidor do produto**: habilite `Access-Control-Allow-Origin: ${NEXT_PUBLIC_CONSOLE_ORIGIN}` e `Access-Control-Allow-Headers: content-type, authorization`.

* * *

10.2. Tipos (alinha com o backend)
==================================

Crie `lib/types.ts`:

```ts
export type Verdict = "Allow" | "Deny" | "Ask";

export interface Metric {
  step: string;
  metric: string;
  value: number;
}

export interface EvaluateResponse {
  product: string;
  verdict: Verdict;
  receipt_cid: string;      // "b3:..."
  url_rica: string;         // resolvível em /r/{cid}
  stopped_at: string | null;
  artifacts: number;
  metrics: Metric[];
  receipt_chain: string[];
}

export interface ExecutionRow {
  id: string;
  state: "ACK" | "NACK" | "ASK";
  cid: string;
  title: string;
  origin: string;
  timestamp: string;  // ISO
  integration: string;
}

export interface ResolverJson {
  // Estrutura mínima para timeline/SIRP derivada do recibo:
  cid: string;
  product?: string;
  verdict?: Verdict;
  hops: Array<{
    step: string;            // ex.: "cap-intake"
    signer?: string;         // opcional
    ts?: string;             // ISO
    hash?: string;           // b3:...|sha256:...
    verified?: boolean;
  }>;
  evidence?: Array<{
    kind: string;            // "artifact" | "log" | "attachment"
    cid?: string;            // se houver
    url?: string;            // se publicado
    summary?: string;
  }>;
}
```

> Esses tipos cobrem exatamente o que a UI precisa para substituir os mocks (execuções, timeline, provas).

* * *

10.3. Cliente Tipado: `lib/env.ts` e `lib/api.ts`
=================================================

`lib/env.ts`:

```ts
export const PRODUCT_BASE = process.env.NEXT_PUBLIC_PRODUCT_BASE_URL!;
export const REGISTRY_BASE = process.env.NEXT_PUBLIC_REGISTRY_BASE_URL!;
```

`lib/api.ts`:

```ts
import { PRODUCT_BASE, REGISTRY_BASE } from "./env";
import type { EvaluateResponse, ExecutionRow, ResolverJson } from "./types";

/** POST /evaluate no produto */
export async function evaluate(input: Record<string, unknown>): Promise<EvaluateResponse> {
  const res = await fetch(`${PRODUCT_BASE}/evaluate`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(input),
  });
  if (!res.ok) throw new Error(`evaluate failed: ${res.status}`);
  return res.json();
}

/** GET /r/{cid} no registry (JSON p/ timeline/SIRP) */
export async function fetchReceiptJson(cid: string): Promise<ResolverJson> {
  const res = await fetch(`${REGISTRY_BASE}/r/${encodeURIComponent(cid)}`, {
    headers: { "accept": "application/ai-nrf1+json, application/json;q=0.9" },
    cache: "no-store",
  });
  if (!res.ok) throw new Error(`resolver failed: ${res.status}`);
  return res.json();
}

/** URL HTML pronto (para iframe/“abrir em nova aba”) */
export function receiptHtmlUrl(cid: string): string {
  return `${REGISTRY_BASE}/r/${encodeURIComponent(cid)}`;
}

/** Lista de execuções (estratégia híbrida) */
export async function listExecutions(): Promise<ExecutionRow[]> {
  // 1) Tente proxy local (ver 10.4). Se não existir, caia no localStorage.
  try {
    const res = await fetch(`/api/executions`, { cache: "no-store" });
    if (res.ok) return res.json();
  } catch {}
  const raw = globalThis.localStorage?.getItem("exec_history");
  return raw ? JSON.parse(raw) : [];
}

/** Persiste a última execução localmente (para a lista) */
export function persistExecution(row: ExecutionRow) {
  const k = "exec_history";
  const current = globalThis.localStorage?.getItem(k);
  const xs: ExecutionRow[] = current ? JSON.parse(current) : [];
  xs.unshift(row);
  const top = xs.slice(0, 200); // guarda os 200 últimos
  globalThis.localStorage?.setItem(k, JSON.stringify(top));
}
```

* * *

10.4. API Routes (proxy opcional)
=================================

Para não depender do registry ter “/search”, adicione um proxy simples que só retorna o histórico local ou, se existir, consulta um endpoint futuro (ex.: `GET /executions` do produto/registry).

`app/api/executions/route.ts`:

```ts
import { NextResponse } from "next/server";

export async function GET() {
  // Hoje apenas devolve vazio (deixe o client cair no localStorage).
  // Opcional: integrar quando houver /executions no produto/registry.
  return NextResponse.json([]);
}
```

> Isso permite manter o `fetch('/api/executions')` sem quebrar SSR.

* * *

10.5. Troca dos Mocks nas Páginas
=================================

A) **Executions** (`app/console/executions/page.tsx`)
-----------------------------------------------------

*   Substituir os imports de `mockExecutions` por chamadas a `listExecutions()`.
*   Ao **submeter** um pedido, chamar `evaluate()`, persistir um `ExecutionRow` e navegar para `/console/r/[cid]`.

Trecho essencial (cliente):

```tsx
"use client";
import { useEffect, useState } from "react";
import { evaluate, listExecutions, persistExecution } from "@/lib/api";
import type { ExecutionRow } from "@/lib/types";
import { useRouter } from "next/navigation";

export default function ExecutionsPage() {
  const [rows, setRows] = useState<ExecutionRow[]>([]);
  const [loading, setLoading] = useState(true);
  const router = useRouter();

  useEffect(() => {
    listExecutions().then(setRows).finally(() => setLoading(false));
  }, []);

  async function onSubmit(form: FormData) {
    const payload = Object.fromEntries(form.entries());
    const res = await evaluate(payload);
    // cria “linha” para tabela
    const row: ExecutionRow = {
      id: crypto.randomUUID(),
      state: res.verdict === "Allow" ? "ACK" : res.verdict === "Deny" ? "NACK" : "ASK",
      cid: res.receipt_cid,
      title: payload["title"]?.toString() || res.product,
      origin: "console",
      integration: "manual",
      timestamp: new Date().toISOString(),
    };
    persistExecution(row);
    router.push(`/console/r/${encodeURIComponent(res.receipt_cid)}`);
  }

  // ...renderização da tabela permanece igual; troque a fonte de dados
}
```

B) **Resolver do recibo** (`app/console/r/[cid]/page.tsx`)
----------------------------------------------------------

*   Remover `mockSIRPNodes/mockProofs` e usar `fetchReceiptJson(cid)`.
*   Exibir botão “Abrir versão HTML” com `receiptHtmlUrl(cid)`.

Trecho essencial:

```tsx
"use client";
import { useEffect, useState } from "react";
import { fetchReceiptJson, receiptHtmlUrl } from "@/lib/api";
import type { ResolverJson } from "@/lib/types";
import Link from "next/link";
import { Button } from "@/components/ui/button";

export default function ReceiptPage({ params }: { params: { cid: string } }) {
  const [data, setData] = useState<ResolverJson | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    fetchReceiptJson(params.cid).then(setData).catch(e => setErr(String(e)));
  }, [params.cid]);

  if (err) return <div className="text-red-500">Erro: {err}</div>;
  if (!data) return <div>Carregando...</div>;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Recibo {data.cid}</h1>
        <Button asChild variant="outline">
          <Link href={receiptHtmlUrl(data.cid)} target="_blank" rel="noreferrer">
            Abrir versão HTML
          </Link>
        </Button>
      </div>

      {/* Timeline SIRP — adapte o componente para consumir data.hops */}
      {/* Evidências/Artifacts — a partir de data.evidence */}
    </div>
  );
}
```

> **Dica**: se o componente `TimelineSIRP` esperava mocks, crie um adapter leve que mapeia `data.hops` para o shape de props atual (passo, signer, verified, ts, hash).

* * *

10.6. Aceite do Console (E2E feliz)
===================================

1.  **Executar** o bin do produto (ex.: porta 8080) e o **registry** (porta 8791).
2.  `npm run dev` no frontend.
3.  Em **/console/executions**:
    *   Preencha o formulário de input e envie → **chama** `POST /evaluate`.
    *   Redireciona para **/console/r/\[cid\]** com **timeline real**.
    *   Botão “Abrir versão HTML” abre o Resolver (`/r/{cid}`) do registry.
4.  **CORS**: sem erros no console; requisições 200/2xx.

* * *

10.7. Ajustes no Backend para facilitar a UI
============================================

*   **Produto**:
    *   Responder JSON de `POST /evaluate` no **formato `EvaluateResponse`** (campos já saem da CLI).
    *   **Duração** e **métricas** por step (já temos no runner).
    *   **CORS**: `Access-Control-Allow-Origin` = origem do Console, `Allow-Headers` = `content-type, authorization`, `Allow-Methods` = `GET,POST,OPTIONS`.
*   **Registry**:
    *   `GET /r/{cid}`:
        *   `Accept: application/ai-nrf1+json` → retornar **ResolverJson** (com `hops` e `evidence`).
        *   `Accept: text/html` → renderizar **status page**.
    *   (Opcional) `GET /search?product=...&limit=...` → lista recibos recentes; facilita o **/api/executions** no futuro.

* * *

10.8. Segurança & UX
====================

*   **Nunca** incluir segredos/keys em recibos/artifacts. 🔒
*   Truncar CIDs longos em chips (com copy-to-clipboard).
*   `try/catch` em fetch com mensagens amigáveis (rede offline, 5xx, timeouts).
*   **Graceful degradations**:
    *   Se `/r/{cid}` JSON falhar, mostre “Abrir HTML” (muitas vezes o HTML do registry ajuda o auditor).
    *   Mantenha o histórico local para o usuário enxergar suas últimas execuções.

* * *

10.9. Smoke Manual (rápido)
===========================

```bash
# 1) Produto
curl -sS -X POST "$NEXT_PUBLIC_PRODUCT_BASE_URL/evaluate" \
  -H 'content-type: application/json' \
  -d '{"data":"hello"}' | jq

# 2) Resolver (JSON)
curl -sS -H 'accept: application/ai-nrf1+json' \
  "$NEXT_PUBLIC_REGISTRY_BASE_URL/r/b3:SEU_CID_AQUI" | jq

# 3) Resolver (HTML)
open "$NEXT_PUBLIC_REGISTRY_BASE_URL/r/b3:SEU_CID_AQUI"
```

* * *

10.10. Roadmap rápido de “troca de mocks” (checklist)
=====================================================

*    Criar `lib/types.ts`, `lib/env.ts`, `lib/api.ts`.
*    Trocar uso de `mockExecutions` por `listExecutions()` e `persistExecution()`.
*    Alterar “submit” em **executions** para usar `evaluate()` e redirecionar para `/console/r/[cid]`.
*    Adaptar **/console/r/\[cid\]** para `fetchReceiptJson(cid)` e exibir HTML via `receiptHtmlUrl`.
*    Habilitar **CORS** no produto.
*    Confirmar content negotiation no **registry**.
*    Ver **TimelineSIRP**/**CardProva**: garantir que aceitam o **shape real** (ou usar adapter).
*    Fumaça manual + screenshot do fluxo.

* * *


Objetivo: empoderar criação de novos produtos só com manifests.
Entregáveis:

Template de product.json.

“Receitas” (AI Model Passport, Construction Draw, Medical Calibration).

Tabela de capabilities e parâmetros com exemplos.
Aceite: alguém fora do core cria um produto end-to-end em minutos.


* * *

Guia do Desenvolvedor de Produto (No-Code/Low-Code)
===================================================

0) O que é um “Produto” aqui?
-----------------------------

Um **Produto** é um _binário gerado_ a partir de:

*   `product.json` (metadados + runtime/server + rotas),
*   1+ manifests de pipeline (`*.yaml|*.json`, executados via **tdln**),
*   `modules.lock.json` (pins por **CID** dos módulos/cfg — “pinned by math”).

> “The Base is the Truth. The Modules are the Tools. The Product is just a JSON file.” ✨

* * *

1) Template de `product.json`
-----------------------------

```jsonc
{
  "schema": "ai-nrf1/product@1",
  "name": "api-receipt-gateway",
  "version": "1.0.0",
  "description": "Gateway que recebe dados, aplica política e retorna recibo + URL rica.",
  "entrypoints": {
    // rota → manifesto
    "/evaluate": { "manifest": "manifests/tdln/policy.yaml" },
    "/runtime":  { "manifest": "manifests/tdln/runtime.yaml" },
    "/llm/engine": { "manifest": "manifests/llm/engine.yaml" },
    "/llm/smart":  { "manifest": "manifests/llm/smart.yaml" }
  },
  "server": {
    "port": 8080,
    "limits": { "max_body_bytes": 1048576, "timeout_ms": 15000 },
    "cors": { "allow_origin": ["http://localhost:3000"], "allow_headers": ["content-type","authorization"] },
    "graceful_shutdown_ms": 3000,
    "structured_logs": true
  },
  "runtime": {
    "backend": "in-process",     // ou "registry"
    "registry_base": "http://127.0.0.1:8791" // se backend=registry
  },
  "locks": "modules.lock.json",  // pins por CID
  "defaults": {
    "vars": { "tenant": "lab512", "priority": "normal" }
  },
  "env_requirements": {
    // Key material / providers utilizados em runtime
    "OPENAI_API_KEY": { "optional": true },
    "ANTHROPIC_API_KEY": { "optional": true },
    "OLLAMA_BASE_URL": { "default": "http://127.0.0.1:11434" }
  }
}
```

> **Entrypoints** mapeiam rotas HTTP → manifests.  
> `locks` aponta para o **modules.lock.json** que fixa módulos/config por hash.

* * *

2) `modules.lock.json` (exemplo mínimo)
---------------------------------------

```json
{
  "schema": "ai-nrf1/modules.lock@1",
  "generated_at": "2026-02-12T11:45:00Z",
  "pins": [
    { "kind": "cap-intake",   "cid": "b3:aa…01", "config_cid": "b3:bb…02" },
    { "kind": "cap-policy",   "cid": "b3:cc…03", "config_cid": "b3:dd…04" },
    { "kind": "cap-runtime",  "cid": "b3:ee…05", "config_cid": "b3:ff…06" },
    { "kind": "cap-llm",      "cid": "b3:11…07", "config_cid": "b3:22…08" },
    { "kind": "cap-enrich",   "cid": "b3:33…09", "config_cid": "b3:44…0a" },
    { "kind": "cap-transport","cid": "b3:55…0b", "config_cid": "b3:66…0c" }
  ]
}
```

> Gerar com `scripts/bootstrap_product.sh` (ver Parte 8/12): computa CIDs de binários e configs e preenche os pins.

* * *

3) Receitas (No-Code) — 3 Produtos Canônicos
--------------------------------------------

### 3.1. **AI Model Passport** (Flagship / EU AI Act)

**Objetivo**: Receber _model card_ + evidências, validar com política (`eu-ai-act.yaml`), opcionalmente executar verificação em **Certified Runtime**, gerar recibo + URL rica.

**product.json (trecho relevante)**

```jsonc
{
  "name": "ai-model-passport",
  "entrypoints": {
    "/evaluate": { "manifest": "manifests/passport/policy.yaml" },
    "/runtime/verify": { "manifest": "manifests/passport/runtime.yaml" }
  }
}
```

**manifests/passport/policy.yaml**

```yaml
version: 1
name: "passport-policy"
includes: []
backend: { mode: in-process }

pipeline:
  - use: cap-intake
    with:
      map:
        - from: "modelCard"   # JSON do form
          to: "payload.modelCard"
        - from: "evidence"    # anexos (hashes/URLs)
          to: "payload.evidence"

  - use: cap-policy
    with:
      families:
        - id: "eu-ai-act"
          rules:
            - kind: "EXIST"
              path: ["payload","modelCard","intendedUse"]
            - kind: "THRESHOLD_RANGE"
              metric: "eval.bias_score"
              min: 0.0
              max: 0.2
            - kind: "EXIST"
              path: ["payload","evidence","traceability"]
      on_deny: { stop: true }

  - use: cap-enrich
    with:
      providers:
        - "status:timeline"
        - "tags:ai-act"

  - use: cap-transport
    with:
      node: "did:ubl:passport"
      relay: []
```

**manifests/passport/runtime.yaml**

```yaml
version: 1
name: "passport-runtime-verify"
backend: { mode: in-process }

pipeline:
  - use: cap-intake
    with:
      map:
        - from: "sourceData"
          to: "payload.source"
        - from: "userCode"    # ex.: checagem determinística do card
          to: "payload.code"

  - use: cap-runtime
    with:
      sandbox: "wasm32-wasi"
      limits: { max_cpu_ms: 500, max_mem_bytes: 33554432 }
      io: { allow_fs: false, allow_net: false }
      attest: { self: true }

  - use: cap-policy
    with:
      families:
        - id: "eu-ai-act-post"
          rules:
            - kind: "EXIST"
              path: ["runtime","attestation","ok"]
      on_deny: { stop: true }

  - use: cap-enrich
    with: { providers: ["status:timeline"] }

  - use: cap-transport
    with: { node: "did:ubl:passport", relay: [] }
```

* * *

### 3.2. **Construction Draw Inspector**

**Objetivo**: Receber _construction drawings_ (metadados/links), validar “completude mínima” e riscos (policy), gerar explicações via **LLM Smart** quando permitido, emitir recibo.

**entrypoints (trecho)**

```jsonc
{
  "name": "construction-draw-inspector",
  "entrypoints": {
    "/evaluate": { "manifest": "manifests/draws/policy-and-smart.yaml" }
  }
}
```

**manifests/draws/policy-and-smart.yaml**

```yaml
version: 1
name: "draws-policy-smart"
backend: { mode: in-process }

pipeline:
  - use: cap-intake
    with:
      map:
        - from: "drawPackage.id"
          to: "payload.packageId"
        - from: "drawPackage.urls"
          to: "payload.urls"

  - use: cap-policy
    with:
      families:
        - id: "draws-minimum"
          rules:
            - kind: "EXIST"
              path: ["payload","urls",0]    # pelo menos 1 arquivo
            - kind: "EXIST"
              path: ["payload","packageId"]
      on_deny: { stop: true }

  - use: cap-llm
    with:
      engine: "smart"               # local/ollama
      model: "ollama:llama3.1:8b"
      prompt_template: |
        Você é um inspetor. Liste até 5 riscos óbvios nas plantas:
        {{payload.urls}}
      max_tokens: 512
      cache_by_cid: true            # cache determinístico por CID do input

  - use: cap-enrich
    with: { providers: ["status:timeline","tags:risk"] }

  - use: cap-transport
    with: { node: "did:ubl:draws", relay: [] }
```

* * *

### 3.3. **Medical Calibration**

**Objetivo**: Verificar calibração de equipamento a partir de leituras + script de validação em **WASM**; impor limites (policy) e emitir recibo para auditoria.

**entrypoints (trecho)**

```jsonc
{
  "name": "medical-calibration",
  "entrypoints": {
    "/evaluate": { "manifest": "manifests/med/calibration.yaml" }
  }
}
```

**manifests/med/calibration.yaml**

```yaml
version: 1
name: "med-calibration"
backend: { mode: in-process }

pipeline:
  - use: cap-intake
    with:
      map:
        - from: "device.id"
          to: "payload.deviceId"
        - from: "readings"
          to: "payload.readings"
        - from: "validatorWasm"
          to: "payload.code"

  - use: cap-runtime
    with:
      sandbox: "wasm32-wasi"
      limits: { max_cpu_ms: 800, max_mem_bytes: 67108864 }
      io: { allow_fs: false, allow_net: false }
      attest: { self: true }

  - use: cap-policy
    with:
      families:
        - id: "calibration-range"
          rules:
            - kind: "THRESHOLD_RANGE"
              metric: "runtime.result.deviation_ppm"
              min: -100
              max: 100
      on_deny: { stop: true }

  - use: cap-enrich
    with: { providers: ["status:timeline","tags:medical"] }

  - use: cap-transport
    with: { node: "did:ubl:med", relay: [] }
```

* * *

4) Tabela de **Capabilities** e Parâmetros (com exemplos)
---------------------------------------------------------

> **Observação**: `with:` é o bloco de parâmetros. Campos ausentes usam _defaults_ seguros.

| Capability | Para quê? | Parâmetros chave (`with`) | Exemplo rápido |
| --- | --- | --- | --- |
| `cap-intake` | Normalizar entrada (JSON→NRF), mapear chaves | `map: [{ from: "a.b", to: "payload.c" }, …]` | Mapear `data` → `payload` |
| `cap-policy` | Decidir **Allow/Deny/Ask** | `families:[{ id, rules:[{kind, …}] }]; on_deny:{stop:bool}` | `EXIST`, `THRESHOLD_RANGE` |
| `cap-runtime` | Executar código do usuário em WASM sandbox | `sandbox:"wasm32-wasi"; limits:{max_cpu_ms,max_mem_bytes}; io:{allow_fs,allow_net}; attest:{self:bool}` | Verificação determinística |
| `cap-llm` | Chamadas LLM (Engine/Smart) | \`engine:"engine" | "smart"; model; max\_tokens; prompt\_template; cache\_by\_cid:bool\` |
| `cap-enrich` | Status page, tags, métricas visuais | `providers:["status:timeline","tags:…","pii-scan"]` | Timeline/SIRP |
| `cap-transport` | Emissão/encadeamento de recibos (URL rica) | `node:"did:ubl:…"; relay:[…]; publish:["receipt","artifacts"]` | `url_rica` |
| `cap-permit` | Permissões/assinaturas (quando aplicável) | `issuer_did; subject; scope; exp_nanos` | Autorizações |
| `cap-pricing` | Precificar/quote/invoice | `catalog_id; rules:[…]; currency` | Custos por step |

> **Dica**: Sempre que usar `cap-llm`, habilite `cache_by_cid: true` para reduzir custo/latência em explicações determinísticas.

* * *

5) Como **gerar** e **rodar** um produto (sem código)
-----------------------------------------------------

1.  **Criar pasta** do produto:
    ```
    products/ai-model-passport/
      ├─ product.json
      ├─ modules.lock.json
      └─ manifests/
          └─ passport/ (policy.yaml, runtime.yaml)
    ```
2.  **Gerar locks**:
    ```
    scripts/bootstrap_product.sh products/ai-model-passport/product.json \
      --out products/ai-model-passport/modules.lock.json
    ```
3.  **Build** do binário:
    ```
    ubl product from-manifest --product products/ai-model-passport/product.json \
      --out target/bin/ai-model-passport
    ```
4.  **Subir servidor**:
    ```
    ./target/bin/ai-model-passport --port 8080
    ```
5.  **Chamar**:
    ```
    curl -sS -X POST 'http://127.0.0.1:8080/evaluate' \
      -H 'content-type: application/json' \
      -d '{"modelCard":{…},"evidence":{…}}' | jq
    ```
6.  **Abrir recibo** (HTML):
    *   Use a `url_rica` retornada (ex.: `http://127.0.0.1:8791/r/b3:…`).

* * *

6) Boas Práticas / Pitfalls
---------------------------

*   **Mapeamento intake**: para chaves simples (ex.: `"payload"`), garanta que o _set_ **insere** em raiz (bug já corrigido).
*   **Policy**: mantenha regras pequenas e específicas; use `on_deny: { stop: true }` para evitar custos desnecessários (LLM/Runtime).
*   **Runtime WASM**: **sem I/O** por padrão (`allow_fs=false`, `allow_net=false`). Deixe explícito o que for permitido.
*   **LLM**:
    *   `engine="engine"` → premium (OpenAI/Anthropic)
    *   `engine="smart"` → local (Ollama).
    *   **Sempre** `cache_by_cid: true` em explicações.
*   **Transport**: defina `node` estável (ex.: `did:ubl:passport`) para facilitar trilhas de auditoria.
*   **Locks por CID**: gere sempre que mudar bundle/manifest/config. _Pins_ garantem reprodutibilidade.
*   **CORS**: se a UI for consumir diretamente, configure `allow_origin`.

* * *

7) Critérios de Aceite (objetivo da Parte 11)
---------------------------------------------

*   Uma pessoa fora do core:
    1.  Copia o **template** de `product.json`;
    2.  Escreve **1–2 manifests** usando a tabela de capabilities;
    3.  Gera locks (`modules.lock.json`);
    4.  Executa `ubl product from-manifest …` e sobe o bin;
    5.  Faz `curl /evaluate` e recebe `receipt_cid` + `url_rica`;
    6.  Abre o recibo no **Resolver** e vê a **timeline** completa.
*   Tempo esperado: **minutos**.
*   Nenhum código Rust escrito.

* * *

8) Checagem Rápida (checklist)
------------------------------

*    `product.json` válido (schema + entrypoints + server + runtime + locks).
*    Manifests semânticos (intake → policy → \[runtime/llm\] → enrich → transport).
*    `modules.lock.json` gerado e com pins para todos os `kind` usados.
*    `POST /evaluate` retorna: `verdict`, `receipt_cid`, `url_rica`, `metrics`, `receipt_chain`.
*    Resolver acessível em `GET /r/{cid}` (HTML e JSON).
*    (Opcional) UI Console integrada (Parte 10).

* * *


Objetivo: lista curta do que fica para v2.
Entregáveis:

Cache-by-CID no cap-llm, GET /r/{cid} full (HTML/NRF), bundles assinados, policy packs versionadas.

Suporte a registry remoto (pull por CID), modules.lock assinado.
Aceite: backlog claro, riscos mapeados, donos definidos.


Parte 12 — Roadmap de Endurecimento (v2)
========================================

Objetivo
--------

Consolidar o “Produto-Factory” para operação real: **imutable builds por CID**, **observabilidade auditável**, **execuções reproduzíveis** e **supply-chain selada**.

* * *

12.1 Cache-by-CID no `cap-llm`
------------------------------

**Por quê**: reduzir custo/latência e assegurar reprodutibilidade quando a entrada (NRF) é idêntica.

**Entregáveis**

*   Hash de _request capsule_ → `cid_req = blake3(nrf_bytes)`.
*   Chave de cache: `{engine, model, max_tokens, prompt_template_cid, cid_req}`.
*   Armazenamento pluggable (FS/local, Redis, S3), TTL e _cardinality guard_.
*   Flag `cache_by_cid: true` + métricas (`cache.hit`, `cache.miss`, `save.ms`).

**DoD**

*   Hit-rate ≥ 60% em suites repetitivas.
*   P95 latência ↓ ≥ 40% em “explain/justify”.
*   Nenhum secret cai no cache (lint de payload).

**Dono**: _LLM & Runtime_  
**Riscos**: colisão de chave por param omitido → **Mitigação**: chave inclui todos os params normalizados.

* * *

12.2 `GET /r/{cid}` completo (HTML + NRF)
-----------------------------------------

**Por quê**: um único endpoint serve auditoria humana (HTML “SIRP”/timeline) e consumo máquina (NRF/JSON).

**Entregáveis**

*   Negotiation por `Accept`: `text/html` → status page; `application/ai-nrf1` → bytes.
*   Página HTML com **timeline** (steps, effects, artifacts) e **cadeia de recibos** (CIDs).
*   _Integrity check_ inline: `verify_integrity(cid)` com badge.

**DoD**

*   `curl -H 'Accept: application/ai-nrf1'` retorna bytes verificáveis.
*   HTML mostra todos os hops + links para artifacts.
*   Teste de snapshot para HTML (estável).

**Dono**: _Registry/Resolver_  
**Riscos**: vazamento de dados sensíveis → **Mitigação**: _redactor_ server-side + allowlist de fields.

* * *

12.3 Bundles assinados
----------------------

**Por quê**: mover de “config arbitrária” para **supply-chain confiável**.

**Entregáveis**

*   Formato `bundle.nrfz` (`.zip`/`.tar.zst`) com:
    *   `bundle.json` (metadados), `manifests/*`, `locks/*`, `SIGNATURE`.
*   Assinatura Ed25519 (chave de _publisher_).
*   CLI: `ubl bundle sign|verify`.

**DoD**

*   `ubl bundle verify <file>` → OK/FAIL com fingerprint do publisher.
*   CI recusa bundle sem assinatura válida (branch protegida).

**Dono**: _Platform_  
**Riscos**: gestão de chaves → **Mitigação**: KMS + rotação semântica (kid no cabeçalho).

* * *

12.4 Policy Packs versionadas
-----------------------------

**Por quê**: políticas reutilizáveis (ex.: `eu-ai-act`) com versionamento e compatibilidade.

**Entregáveis**

*   Estrutura: `policy-packs/<name>/<semver>/pack.yaml`.
*   Manifests referenciam `policy_pack: eu-ai-act@^1`.
*   Changelog + testes de compat (fixtures).

**DoD**

*   `ubl policy-pack lint/test` verde com fixtures de exemplo.
*   _Semver guard_: _breaking_ exige major bump.

**Dono**: _Risk & Policy_  
**Riscos**: “policy drift” entre produtos → **Mitigação**: lock por CID + _compat tests_ cruzados.

* * *

12.5 Suporte a **Registry Remoto** (pull por CID)
-------------------------------------------------

**Por quê**: distribuir módulos/manifests por **CID**, não por branch/tag.

**Entregáveis**

*   API: `GET /modules/{cid}`, `GET /configs/{cid}`.
*   Runner “registry” backend com cache local e _trust store_ (publishers permitidos).
*   Prefetch/Prewarm (`ubl prefetch --cids ...`).

**DoD**

*   Airgap-friendly com _mirror local_.
*   Fallback offline (cache) quando registry indisponível.

**Dono**: _Registry & Runner_  
**Riscos**: latência de cold-start → **Mitigação**: prefetch + LRU local.

* * *

12.6 `modules.lock.json` **assinado**
-------------------------------------

**Por quê**: garantir que o conjunto exato de módulos/config foi usado (SLSA-like).

**Entregáveis**

*   `modules.lock.sig` (Ed25519), embed do `kid`.
*   Validação na inicialização do binário gerado (fail-closed).
*   CLI: `ubl lock sign|verify`.

**DoD**

*   Boot falha se lock não verificar.
*   Resolver exibe “Assinado por … (kid …)” no HTML.

**Dono**: _Platform & Security_  
**Riscos**: _key loss_ → **Mitigação**: HSM/KMS + _emergency-rekey_ documentado.

* * *

Trilha de Execução (v2)
-----------------------

1.  **Infra segura**
    *   Lock assinado (12.6) → Bundles assinados (12.3).
2.  **Operacional**
    *   `GET /r/{cid}` full (12.2) → Registry remoto (12.5).
3.  **Eficiência**
    *   Cache-by-CID no `cap-llm` (12.1).
4.  **Domínio**
    *   Policy packs versionadas (12.4).

* * *

Métricas de Sucesso
-------------------

*   100% dos binários de produto sobem **apenas** com `modules.lock` assinado.
*   ≥ 95% das requisições de recibo servidas via `GET /r/{cid}` (sem erros).
*   Cache LLM: **P95** ↓ ≥ 40% em rotas de explicação; **custo** ↓ ≥ 35%.
*   0 vazamentos de segredo em recibos/artifacts (scanner CI/Runtime).

* * *

Riscos & Mitigações (resumo)
----------------------------

*   **Chaves**: perda/rotação → KMS/HSM + _kid_ + playbook de rotação.
*   **Drift** de política → packs versionadas + compat tests.
*   **Cold start** do registry → prefetch + cache local.
*   **Regressões HTML** → snapshot tests e contrato `Accept`.

* * *

Donos (por capítulo)
--------------------

*   **LLM & Runtime**: 12.1 (cache LLM)
*   **Registry/Resolver**: 12.2 (GET /r), 12.5 (pull por CID)
*   **Platform**: 12.3 (bundles assinados), 12.6 (lock assinado)
*   **Risk & Policy**: 12.4 (packs versionadas)

* * *

Aceite da Parte 12
------------------

*   Backlog v2 publicado com estes 6 épicos, donos e métricas.
*   _Runbooks_ de chaves e resposta a incidentes anexados.
*   PoC de cada item demonstrável (CLI/HTTP) e testado em CI (gates de segurança habilitados).
