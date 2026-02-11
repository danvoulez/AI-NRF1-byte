# Module Architecture — Design Document

> Extracted from MODULOS-INICIO.md.
> Architectural rationale and design decisions for the MODULE phase.
> Code lives in separate extracted files.

---

## Original Vision: 5 Canonical Modules


Com a BASE pronta (bytes canônicos, ID estável, Seal, Receipts), o perigo agora é criar um módulo para cada "feature" (ex: mod-email, mod-slack, mod-pdf-finra). Isso é o caos.
Para evitar a explosão, você precisa de Módulos de Capacidade (Engines), não de Módulos de Caso de Uso. O "Caso de Uso" é apenas configuração (data/json) injetada no módulo.
Aqui estão os 5 Módulos Canônicos que resolvem 90% dos produtos sem você escrever código novo:
1. mod-intake (O Normalizador)
Missão: Transformar "barulho do mundo" (JSON sujo, Webhooks, Docs) em env canônico (ai-json-nrf1).
 * Por que evita explosão: Em vez de stripe-intake, github-intake, você faz um módulo que aceita um Schema de Mapeamento.
 * Configuração: JSONPath ou jq-like query.
 * Saída: CanonValue (o env limpo).
 * Tipos:
   * Document (blob estático).
   * Event (stream temporal).
   * Transaction (acordo bilateral).
2. mod-policy-engine (O Juiz)
Missão: Receber o env, rodar regras e cuspir ALLOW, DENY, REQUIRE + Evidence.
 * Por que evita explosão: Você não cria mod-policy-compliance-bancario. Você cria um engine que carrega Policy Packs (arquivos .wasm ou .json lógica).
 * Famílias Suportadas:
   * Existence: "Campos X, Y existem?"
   * Threshold: "Score > 0.8?" (Sem float no canon, use int escalado).
   * Allowlist/Blocklist: "DID está na lista?"
   * Logic: "Se A e B então C".
 * O Pulo do Gato: O módulo é burro. A inteligência está no Pack (que é versionado e assinado na Base).
3. mod-permit (O Cartório / Consent)
Missão: Resolver o estado REQUIRE. Gerenciar filas de aprovação humana (K-of-N) e emitir o Permit assinado.
 * Por que evita explosão: Gerencia qualquer tipo de aprovação (Financeira, Legal, Técnica) usando a mesma lógica de criptografia e TTL.
 * Funcionalidade:
   * Recebe REQUIRE do Policy Engine.
   * Cria "Ticket" de consentimento.
   * Aguarda K assinaturas de N roles.
   * Troca Ticket por Permit (assinatura Ed25519 sobre hash do input).
4. mod-enrich (O Narrador)
Missão: Pegar o Recibo (frio e matemático) e gerar artefatos para humanos/sistemas.
 * Por que evita explosão: Em vez de um módulo pra PDF, outro pra HTML, outro pra Webhook, você tem um Renderizador baseado em templates.
 * Saídas (Drivers):
   * Status Page: HTML dinâmico no card_url.
   * Badge: SVG (verde/vermelho).
   * Notification: Webhook assinado (JSON) para sistemas externos.
   * Ghost: Versão "redacted" do recibo (esconde segredos, mostra provas).
5. mod-llm-assist (O Estagiário Determinístico)
Missão: Ajudar os outros módulos onde a regra rígida falha, mas sempre confinado pelo ai-nrf1.
 * Por que evita explosão: Evita escrever código complexo de parsing de texto.
 * Usos:
   * No mod-intake: Extrair campos estruturados de um PDF.
   * No mod-enrich: Escrever a "Reasoning" humana do NACK ("Negado porque faltou o documento X").
   * Regra de Ouro: O LLM sugere, a Base assina. O output do LLM entra no pipeline como dado, não como decisão final.
Como isso monta os Produtos (Exemplos)
Com apenas esses 5 módulos, você monta:
| Produto | Configuração (Manifesto) |
|---|---|
| API Gateway | Intake(Transaction) → Policy(RateLimit + Auth) → Enrich(Webhook) |
| Model Passport | Intake(Doc) → Policy(EU-AI-Act) → Enrich(Badge + Status Page) |
| Underwriting | Intake(Doc) → Policy(Risk) → Permit(K=2, N=3) → Enrich(PDF) |
Resumo da Arquitetura
 * BASE (Rust): A verdade física (bytes, hash, assinatura).
 * MÓDULOS (Rust/Wasm): As 5 máquinas de processamento (Intake, Policy, Permit, Enrich, LLM).
 * PRODUTO (Config/Bin): O manifesto JSON que liga os fios.
Sua tarefa agora: Fazer o scaffold desses 5 módulos. Comece pelo mod-intake e mod-policy-engine. O resto é consequência.

---

## Refined Architecture: Capabilities + Orchestrator

Dan — o teu plano está **muito bom**. Ele acerta o ponto mais perigoso (explosão de “módulo por caso de uso”) e troca isso por **módulos de capacidade** com **caso de uso = dados (manifesto/packs/templates)**. Isso é exatamente o que deixa a BASE virar plataforma. 🔥

A minha versão só faz **3 apertos** pra ficar “à prova de crescimento”:

1.  **Eu não faria “5 módulos” como bins separados logo de cara**.  
    Eu faria **5 capacidades** como _crates_ (bibliotecas) + **1 orquestrador** (um bin “hub”) que compõe tudo por manifesto.  
    Motivo: evita overhead de IPC, versionamento e deploy de N serviços. Você ganha modularidade sem virar microserviço cedo demais.
2.  Eu separaria **Policy Engine** em duas camadas:
    *   **Evaluator** (determinístico, puro, sem IO): `env → decisão + PoI`
    *   **Runner** (com IO): carrega packs, chama WASM, resolve references, cache, etc.  
        Motivo: isso deixa testes e segurança muito mais fáceis (o “juiz” é _pure function_).
3.  Eu criaria um **módulo 0 “mod-store”** (ou “mod-resolver”) bem pequeno, porque ele impede 80% da bagunça:
    *   resolve `CID → blob` (packs, templates, evidence)
    *   cache
    *   valida assinatura / allowlist de publishers  
        Motivo: sem isso, cada módulo inventa seu jeito de buscar pack/template/evidence e aí explode mesmo.

* * *

Minha versão (mesma ideia, mais “engenharia”)
=============================================

✅ BASE (já)
-----------

*   **ai-nrf1 (bytes/canon)**, **IDs**, **Seal**, **Receipts**, **verify**
*   é a “lei física”.

🧠 CAPACIDADES (crates) — canônicas
-----------------------------------

### 0) `cap-store` (Resolver / Content Addressed IO)

**Missão:** “CID entra, bytes saem (ou erro)”.

*   Backends: FS, S3/MinIO, HTTP, in-memory
*   Políticas: allowlist de publishers, pinning, cache, quotas
*   _Tudo_ que for pack/template/evidence passa aqui.

> Por que é obrigatório: sem esse módulo, Intake/Policy/Enrich duplicam IO e assinatura.

* * *

### 1) `cap-intake` (Normalizer)

**Entrada:** mundo sujo (JSON, webhook, doc, payload).  
**Saída:** `env` canônico (`ai-json-nrf1`) + `evidence` (CIDs).

*   **Sem plugins por fonte**: usa `MappingSpec` (jq/JSONPath-like)
*   Tipos: Document / Event / Transaction
*   Regras hard: NFC/ASCII, sem float, ints escalados

> Seu conceito está perfeito. Eu só amarro ao `cap-store` e exijo que qualquer blob vire CID.

* * *

### 2) `cap-policy` (Evaluator + Runner)

**Evaluator (puro):** `env → verdict (ALLOW/DENY/REQUIRE) + PoI + metrics`  
**Runner (impuro):** carrega packs, executa WASM, resolve links por CID.

*   Packs versionados/assinados na BASE
*   Famílias: existence, threshold(int), allow/block, logic, provenance/auth (quando necessário)
*   Domain separation e determinismo sempre

> Isso impede “mod-policy-compliance-XYZ”. Compliance é pack, não módulo.

* * *

### 3) `cap-permit` (Consent / Cartório)

**Missão:** transformar REQUIRE em “go/no-go” com prova.

*   Ticket + K-of-N + TTL via `hdr.exp`
*   **Emitir Permit**: assinatura sobre hash do input (NRF bytes) + escopo + exp
*   Logs append-only (ledger NDJSON)

> Aqui eu adiciono um detalhe: “permit verify” tem que ser uma função **reutilizada pelo executor**.

* * *

### 4) `cap-enrich` (Renderer / Narrador)

**Missão:** receipt frio → artefatos humanos/sistêmicos

*   Drivers: HTML status page, badge SVG, webhook assinado, ghost redacted, PDF
*   _Tudo template-driven_ (não por produto)
*   Redaction policy declarativa (manifesto)

> Seu “um módulo renderizador” é exatamente a forma certa.

* * *

### 5) `cap-llm-assist` (Copilot confinado)

**Missão:** ajudar, nunca decidir.

*   Entrada/saída sempre em `env`/dados
*   Prompting e tools versionados por CID
*   Guardrails: output assinado só depois de passar por Policy/Permit

> Perfeito do jeito que você colocou: “LLM sugere, BASE assina”.

* * *

🧩 ORQUESTRADOR (1 bin)
-----------------------

### `ai-nrf1d` (ou `ubl-core`)

**Missão:** ler `product.json` e montar o pipeline **no mesmo processo**:  
`intake → policy → (permit?) → enrich → (dispatch)`

*   runtime com feature flags pros caps (ou dynamic “providers”)
*   expõe 1 API HTTP/gRPC opcional
*   pode rodar local/PM2 e virar “sempre ON”

> Isso resolve a tua pergunta “centro de comando vs cada produto compõe”:  
> **Meu default: um centro (orquestrador) compõe capacidades por manifesto.**  
> E se um dia precisar, você extrai um cap pra serviço separado sem quebrar o modelo.

* * *

Produtos (do jeito profissional)
================================

Um “produto” é:

*   `product.json` (manifesto)
*   packs (policy/llm/templates) referenciados por CID
*   às vezes um bin fino (`registry` por ex.) se precisar de API/estado próprio

Ou seja:

*   **Produto ≠ código novo**
*   Produto = **config + assets assinados**

* * *

O que eu mudaria no teu texto (bem pontual)
===========================================

✅ manter 5 módulos/capacidades  
➕ adicionar `cap-store` como “módulo 0” (resolve caos de assets)  
🔧 policy engine dividido em **Evaluator puro** + **Runner**  
🏗️ “módulos” como **crates**, e um **orquestrador único** que compõe por manifesto

* * *

Ordem de scaffold (minha)
=========================

1.  `cap-store` (CID → bytes + cache + verify publisher)
2.  `cap-intake` (MappingSpec + output ai-json-nrf1 + evidence CIDs)
3.  `cap-policy` (Evaluator puro primeiro; Runner depois)
4.  `ai-nrf1d` (orquestrador lendo manifesto)
5.  `cap-enrich` (status+webhook+ghost primeiro)
6.  `cap-permit` (REQUIRE end-to-end)
7.  `cap-llm-assist` (só depois que o determinismo estiver fechado)

* * *

Se você quiser, eu transformo isso em um **manifesto “v0”** bem enxuto com:

*   `pipeline: [steps...]`
*   `packs: [cid...]`
*   `enrich: {...}`
*   `consent: {...}`
*   `store: {...}`

…e isso vira a base do teu comando CLI “criar produto por manifesto” sem precisar inventar UI agora. UI pode ser só um _viewer_ (status page) no começo.


---

## Module Model v0: Traits, Ctx, Contracts

✅ Modelo Canônico de Módulo (Capability) — v0
=============================================

1) Contrato de entrada/saída (sempre igual)
-------------------------------------------

Todo módulo implementa a mesma assinatura mental:

**Input:** `ModuleInput` (dados canônicos + refs)  
**Output:** `ModuleOutput` (novos dados canônicos + artefatos + eventos)

Sem “mutação escondida”. E sem JSON solto: o núcleo sempre trabalha em **ai-nrf1 Value** / **ai-json-nrf1 struct**.

### Tipos base (contrato)

*   `Env` = `ai-json-nrf1` (semântica)
*   `Hdr` = roteamento (quando fizer sentido; módulos de policy não veem `hdr`)
*   `Capsule` (quando já estiver no nível ubl-capsule)
*   `CID` = bytes(32)
*   `Receipt` = output matemático do pipeline (p/ auditoria e enrich)

* * *

2) Estrutura de pastas padrão
-----------------------------

```
modules/<cap-name>/
  Cargo.toml
  src/
    lib.rs
    api.rs          # traits + tipos públicos (estáveis)
    config.rs       # types do manifesto + validações
    core.rs         # lógica pura (determinística)
    io.rs           # adapters/impuro (store/network/time)
    errors.rs
  tests/
    integration.rs  # testes end-to-end do módulo
  vectors/
    *.json          # view humana
    *.nrf           # canon bytes (truth)
```

Regra: **core.rs não importa IO**. `io.rs` é o lugar do “mundo real”.

* * *

3) Trait padrão: `Capability`
-----------------------------

Todo módulo expõe um trait único com 2 fases:

*   `plan()` (opcional): prepara/valida config, resolve refs, pré-carrega pack/template, etc.
*   `execute()` (obrigatório): roda e devolve resultado

```rust
pub trait Capability {
    type Config: serde::DeserializeOwned + Clone + Send + Sync + 'static;
    type Input: Clone + Send + Sync + 'static;
    type Output: Clone + Send + Sync + 'static;

    fn kind(&self) -> &'static str; // "cap-intake", "cap-policy", etc.

    fn plan(&self, _ctx: &Ctx, _cfg: &Self::Config) -> Result<Plan, CapErr> {
        Ok(Plan::default())
    }

    fn execute(&self, ctx: &Ctx, cfg: &Self::Config, input: Self::Input) -> Result<Self::Output, CapErr>;
}
```

`Plan` pode carregar:

*   `resolved_cids: Vec<Cid>`
*   `cache_keys: Vec<String>`
*   `warnings: Vec<String>`

* * *

4) `Ctx`: contexto padrão do runtime (injeção única)
----------------------------------------------------

O módulo **não** faz `std::env`, não lê arquivo direto, não chama rede direto (salvo via store). Ele pede tudo ao `Ctx`.

```rust
pub struct Ctx {
  pub store: Arc<dyn Store>,      // CID -> bytes
  pub clock: Arc<dyn Clock>,      // now_nanos()
  pub keys: Arc<dyn Keyring>,     // sign/verify
  pub audit: Arc<dyn AuditSink>,  // NDJSON append
  pub rng: Arc<dyn Rng16>,        // nonce
}
```

Isso é o que te permite:

*   rodar em CLI
*   rodar em server
*   rodar em WASM (substitui Store/Clock/Keys)
*   testar com fakes

* * *

5) Config padrão (derivado do manifesto)
----------------------------------------

Todo módulo tem `Config` com:

*   `id`: nome lógico (ex.: `"policy_main"`)
*   `inputs`: quais campos do `Env` ele lê
*   `outputs`: o que ele escreve
*   `refs`: CIDs que ele precisa (packs/templates/prompts)
*   `params`: knobs do módulo

Exemplo mínimo:

```json
{
  "id": "policy_main",
  "kind": "cap-policy",
  "refs": { "packs": ["b3:..."] },
  "params": { "mode": "wasm", "strict": true }
}
```

E o módulo valida:

*   refs existem
*   tipos batem
*   invariantes

* * *

6) `ModuleInput` e `ModuleOutput` (modelo unificado)
----------------------------------------------------

Você pode padronizar isso e todo módulo se beneficia:

```rust
pub struct ModuleInput {
  pub env: Env,                 // ai-json-nrf1 typed
  pub capsule: Option<Capsule>, // se já estiver em nível SIRP/UBL
  pub prev: Option<Cid>,        // pipeline_prev
}

pub struct ModuleOutput {
  pub env: Env,
  pub artifacts: Vec<Artifact>, // html/svg/pdf/webhook payload
  pub events: Vec<Event>,       // métricas, logs, etc.
  pub receipts: Vec<Receipt>,   // quando o módulo produz prova
}
```

Cada cap pode usar um `Input/Output` mais específico, mas esse é o “shape mental”.

* * *

7) Regras de ouro do módulo (anti-caos)
---------------------------------------

1.  **Determinismo por default**: qualquer coisa dependente de tempo/aleatório entra via `Ctx`.
2.  **Sem parsing “solto”**: view JSON vira canonical antes de entrar no core.
3.  **Sem decisões em LLM**: LLM só gera _dados_, nunca verdict.
4.  **Sem módulo por caso de uso**: caso de uso = `Config + Packs + Templates`.
5.  **Tudo que é asset é CID**: prompt/template/policypack = content addressed.

* * *

📦 Templates concretos (pra você copiar/colar)
==============================================

A) `cap-intake` (Normalizer) — assinatura típica
------------------------------------------------

*   Config: `MappingSpec { selectors, schema_ref, evidence_rules }`
*   Input: `RawInput { bytes/json, content_type, source }`
*   Output: `Env + evidence.cids`

B) `cap-policy` (Evaluator/Runner)
----------------------------------

*   Config: `PolicyConfig { packs: [Cid], mode: Wasm|Json, strict }`
*   Input: `Env`
*   Output: `Decision { verdict, poi, metrics }` + “receipt candidate” (ainda sem hops)

C) `cap-enrich` (Renderer)
--------------------------

*   Config: `EnrichConfig { drivers, templates, redaction }`
*   Input: `Env + Decision + Receipt`
*   Output: `Artifacts` (html/svg/pdf/webhook payload)

D) `cap-permit` (Consent)
-------------------------

*   Config: `ConsentConfig { kofn, ttl, timeout_action }`
*   Input: `Decision(REQUIRE) + request_hash`
*   Output: `Permit` ou `FinalDecision`

* * *

🧪 Modelo de testes obrigatório por módulo
==========================================

Cada módulo tem **3 níveis**:

1.  **Vectors (KAT)**: entrada JSON view + canon NRF esperado
2.  **Property tests (quando fizer sentido)**: invariantes (ex.: idempotência)
3.  **Integration**: plug no orquestrador com um manifesto mínimo e roda 1 pipeline

Exemplo de invariantes úteis:

*   intake: “mesma entrada → mesmo env”
*   policy: “mesmo env/packs → mesma decisão”
*   enrich: “mesmo receipt → mesmo html/svg (bytes)”
*   permit: “k-of-n → permit válido e verificável”

* * *

🧩 E como isso vira “um binário com feature flags”?
===================================================

O binário (tipo `ai-nrf1d`) compila com:

*   `--features cap-intake,cap-policy,cap-enrich,...`
*   registra os módulos numa `Registry` interna:

```rust
pub struct CapRegistry { map: HashMap<String, Arc<dyn AnyCap>> }
```

e o manifesto escolhe quais instanciar. Se o cap não estiver compilado, erro claro: `Err.Cap.NotAvailable`.

* * *

---

## Refined Model: Effects, Idempotency, Taxonomy

Modelo Canônico de Módulo (Capability) — versão refinada
========================================================

0) Premissas (invariantes do ecossistema)
-----------------------------------------

1.  **Canon = bytes (ai-nrf1 / ubl-byte)**.  
    JSON/YAML = _view_, input humano, ou config. Nunca “verdade”.
2.  **Produto = pipeline declarativo** (manifesto), não um codebase novo.  
    Um “produto” só existe se eu conseguir descrevê-lo como “steps + configs + packs/assets por CID”.
3.  **Módulo é capacidade**, não caso de uso.  
    Ex: `cap-notify` (driver webhook/email/slack) e não `mod-slack-finra`.
4.  **Tudo que for “conteúdo” é asset content-addressed**: policy packs, templates, prompts, schemas, regras, etc.

* * *

1) O formato real do módulo (o que ele é no repo)
-------------------------------------------------

Um módulo é um **crate Rust** (biblioteca), com API estável, e implementação “core puro” separada de IO.

Estrutura recomendada (igual pra todos):

```
modules/<cap_name>/
  src/
    lib.rs         # exports públicos
    api.rs         # traits + tipos públicos (estáveis)
    config.rs      # tipos do manifesto + validação
    core.rs        # determinístico, puro
    io.rs          # adapters do mundo real (store/net/clock)
    errors.rs      # erros padronizados
  tests/
    kat.rs         # vectors / known-answer tests (NRF bytes)
    integration.rs # smoke do cap rodando via runtime
  vectors/
    *.json         # view humana
    *.nrf          # canon bytes (truth)
```

**Regra**: `core.rs` não pode depender de rede, FS, env vars. Só de tipos e funções puras.

* * *

2) O contrato do módulo (um shape só, sempre)
---------------------------------------------

Pra evitar “cada módulo inventa um input/output”, a gente padroniza **um envelope universal**.

### `CapInput`

*   `env`: ai-json-nrf1 (a semântica)
*   `capsule`: opcional (se já está no nível UBL Capsule)
*   `pipeline_prev`: lista de CIDs (composição de atos)
*   `assets`: refs resolvidas (packs/templates/etc), **por CID**
*   `meta`: metadados de execução (run\_id, tenant, trace\_id, etc)

### `CapOutput`

*   `env`: (imutável ou “novo env”, mas sempre explícito)
*   `artifacts`: html/svg/pdf/json (outputs derivados)
*   `effects`: side-effects declarados (ex.: “enviar webhook X”)
*   `receipts`: receipts/capsules gerados (quando aplicável)
*   `metrics/events`: observabilidade e auditoria

O ponto: o módulo pode até ter structs internas próprias, mas **a borda é padronizada**.

* * *

3) Separar “resultado” de “efeito” (isso mata metade do caos)
-------------------------------------------------------------

O que costuma explodir projetos desse tipo é módulo fazendo IO “por baixo”.

Então: **capabilities devem preferir retornar “effects”** em vez de executar.

### Exemplo

*   `cap-enrich` retorna `Artifact::HtmlStatusPage(bytes)` e `Effect::PublishToCardURL { cid, bytes }`
*   o runtime (orquestrador) decide se publica, re-tenta, assina, enfileira etc.

Isso te dá:

*   reprocessamento idempotente
*   retries seguros
*   execução offline e teste fácil
*   “async/await gateway” vira detalhe do runtime, não do módulo

* * *

4) `Ctx` (contexto único), com “ports” bem definidos
----------------------------------------------------

Módulo não conversa com o mundo direto. Conversa via interfaces.

**Ports mínimos:**

*   `Store`: get/put por CID (conteúdo e assets)
*   `Clock`: now\_nanos()
*   `Keyring`: sign/verify (Ed25519/PQ)
*   `AuditSink`: append NDJSON (log à prova de auditoria)
*   `Net`: (opcional) fetch externo (mas idealmente só o runtime usa)
*   `Queue`: (opcional) para efeitos async

Na prática, `Ctx` vira o “hardware” da plataforma.

* * *

5) O que vai no `Config` (derivado do manifesto)
------------------------------------------------

O config deve ser **declarativo** e **validável**.

Campos canônicos:

*   `id`: nome lógico do step (ex.: `"policy_main"`)
*   `kind`: `"cap-policy"`, `"cap-intake"`, etc
*   `assets`: lista de `CidRef` (packs, templates, schema refs…)
*   `params`: knobs do cap (strict, mode, timeouts, redaction etc)
*   `io`: “declaração” de endpoints/drivers (sem segredo; segredo é via env/ctx)

E sempre: `validate()` que checa invariantes e resolve refs.

* * *

6) Versionamento (pra não quebrar produtos)
-------------------------------------------

Cada cap deve declarar:

*   `cap_kind`: `"cap-policy"`
*   `cap_api_version`: `"1.0"`
*   `compat`: lista de versões aceitas do manifesto/config

Regra: **produto aponta pra cap\_kind + cap\_api\_version**, não pra crate.

Assim você consegue:

*   evoluir implementações
*   manter compat de manifesto
*   evitar refactor que quebra tudo

* * *

7) Idempotência e determinismo (a checklist obrigatória)
--------------------------------------------------------

Cada cap deve declarar, explicitamente:

*   **Determinístico?** (mesmo input + mesmos assets + mesma seed → mesmo output)
*   **Idempotente?** (rodar duas vezes não duplica efeito / receipts)
*   **Reentrante?** (pode retomar de uma falha sem drift)
*   **Pure vs Effectful?** (ideal: puro e retorna effects)

E o runtime deve ter:

*   `dedupe_key` por step (ex.: hash do `CapInput` canônico)
*   cache opcional de outputs por `dedupe_key`

* * *

8) Logging / Auditoria (sem vazar segredo)
------------------------------------------

Módulo escreve:

*   `AuditEvent` com hash do input canônico + resumo
*   nunca escreve payload bruto sensível (só CID + redaction)

Padrão:

*   `event.kind`, `event.step_id`, `event.run_id`, `event.input_cid`, `event.output_cid`, `event.verdict` (se existir)

Isso faz teu “ops/audit” virar matemática.

* * *

9) Taxonomia mínima de capabilities (pra evitar 200 módulos)
------------------------------------------------------------

Eu refinaria teus 5 como **famílias**, e não módulos únicos:

### A) `cap-intake` (Normalize)

*   entra “mundo” → sai env canônico
*   mapping spec (jsonpath/jq-like) + schema check
*   pode usar `cap-llm` como auxiliar (mas output vira dado)

### B) `cap-policy` (Decide)

*   env + packs → decision (ACK/NACK/ASK + PoI)
*   packs podem ser wasm/json (mas “pack” é asset por CID)

### C) `cap-permit` (Consent)

*   transforma ASK/REQUIRE em ticket→permit
*   K-of-N, TTL por exp, timeout\_action

### D) `cap-enrich` (Render)

*   receipt/decision → artifacts/effects
*   drivers: html, badge, pdf, webhook, ghost
*   redaction config por manifesto

### E) `cap-transport` (SIRP/Relay)

*   lida com hdr/nonce/exp/receipts hop-chain
*   é a ponte pra “mundo rede”

### F) `cap-llm` (Assist)

*   interface única “ask model”
*   output sempre “proposta”, nunca decisão final
*   guarda prompt/template como asset por CID

Percebe? Eu **não colocaria transport dentro de enrich** e nem “LLM assist” solto: ambos merecem família própria.

* * *

10) Runtime/Orquestrador (onde entra o “gateway async/await”)
-------------------------------------------------------------

Isso é importante: **a bagunça não é resolvida dentro de módulos**. É resolvida no runtime.

O runtime é quem:

*   executa pipeline steps em ordem
*   resolve assets por CID
*   aplica dedupe/idempotência
*   executa effects (sync ou async)
*   mantém state machine (NEW → READY → RUNNING → DONE/ERROR)
*   emite a UBL Capsule e hop receipts se o produto pede “proof\_level: sirp/bundle”

Então: **sim**, é comum precisar de um “gateway async/await” — mas ele é o **runtime executor**, não um módulo ad hoc.

* * *

11) “Core module vs general module” (sem complicar demais)
----------------------------------------------------------

Você não precisa dessa divisão formal agora. Mas dá pra usar um critério simples:

*   **Core caps**: necessários pra qualquer produto (intake, policy, enrich, transport, permit)
*   **Addon caps**: integrações específicas (ex.: driver “smtp”, “slack api”, “s3 store”) — mas isso deve entrar como **drivers** dentro de famílias core, não como cap novo.

Ou seja: _não_ “cap-slack”; sim “cap-enrich(driver=slack)” ou “cap-notify(driver=slack)”.

* * *

Minha versão (resumida) do teu plano
====================================

O teu plano é bom. Minha “versão plataforma” fica:

1.  **cap-intake** (normalize)
2.  **cap-policy** (decide)
3.  **cap-permit** (human gate)
4.  **cap-enrich** (render + effects)
5.  **cap-transport** (SIRP + hop receipts)
6.  **cap-llm** (assist confinado)

E um **runtime** forte que:

*   compõe por manifesto
*   executa effects async
*   garante dedupe/idempotência
*   controla state machine e audit

* * *

Se você curtir, o próximo passo natural (sem pressa) é eu te entregar:

*   um **template “cap-template”** com todos esses arquivos/traits,
*   e um **manifesto mínimo** que instancia 2 steps (intake → policy) e produz um `receipt + artifacts`, sem rede.

Quer que eu refine também a parte “manifest schema” (como declarar pipeline, assets, drivers, proof\_level, and effects) pra ficar 100% fechada com esse modelo?

---

