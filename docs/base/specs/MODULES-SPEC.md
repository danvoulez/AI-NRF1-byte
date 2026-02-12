#2 — MÓDULOS (Core + Domínio + Enriquecimento)

> **Missão:** compor capacidades de alto nível (políticas, mapeadores, verificadores, enriquecimentos) sobre a **BASE** (ai-nrf1 / ai-json-nrf1 / ubl-capsule v1), exportando **APIs estáveis** e **artefatos plug-and-play** para **PRODUTOS**.

---

## 0) Topologia

* **Core Modules** (núcleo, sempre presentes)

  * `intake-*` (document / event / transaction)
  * `policy-*` (existence, threshold, compliance, provenance, authorization)
  * `verify-*` (capsule/chain/offline-bundle)
  * `enrich-*` (card_url, badge, status, pdf, ghost, webhook)
  * `sirp-*` (capsules, delivery, execution, verify)
  * `permit-*` (issue/verify, k-of-n consent)
* **Domain Modules** (por vertical)

  * `schema-*` (tipos/kinds), `mapper-*` (flattening ↔ engine JSON)
  * packs: `pack-finra`, `pack-hipaa`, `pack-eu-ai`, `pack-slsa`, etc.
* **LLM Modules** (otimizadores)

  * `llm-engine` (orquestração determinística TDLN-first)
  * `llm-smart` (heurísticas/recursos premium/OSS, caching, RAG controlado)

**Entrega:** 1 binário multi-módulo (feature flags) **ou** libs estáticas/Dynamic-plugins (WASM).
**Destino:** cada **PRODUTO** seleciona módulos (manifesto) e compõe pipelines.

---

## 1) Contratos de integração

### 1.1 ABI (WASM / FFI) — “puro buffer”

* Entrada/saída sempre **ai-nrf1** (canon); **sem I/O interno**.

```text
fn intake(in_ai_nrf1: &[u8])         -> Result<Vec<u8>>  // normaliza entrada → env.ctx/intent
fn evaluate(in_capsule_ai_nrf1: &[u8])-> Result<Vec<u8>> // aplica policies → decision + evidence
fn enrich(in_capsule_ai_nrf1: &[u8])  -> Result<Vec<u8>> // badge/pdf/ghost/webhook manifest
fn verify(in_capsule_ai_nrf1: &[u8])  -> Result<Vec<u8>> // chain/assinaturas/invariantes
```

> Observação: cada módulo implementa **uma** função primária; composição é feita pelo **Runtime de Módulos** (abaixo).

### 1.2 Runtime de Módulos (host)

* Orquestra **pipeline** `intake → evaluate → enrich → verify` (configurável).
* Passa/valida **permit** e **k-of-n** quando a decisão = `REQUIRE/ASK`.
* Envolve cada chamada com **WBE/HAL** + **timeouts** + **mem/CPU caps**.

### 1.3 Esquema de mensagens entre módulos

* **Sempre ubl-capsule v1** (ai-nrf1) — **env** é o payload semântico.
* Módulos **NÃO** tocam `hdr`, `id`, `seal`, `receipts` (somente host).
* Em casos de ASK, módulo **deve** preencher `env.links.prev`.

---

## 2) Taxonomia e responsabilidades

### 2.1 Intake (3 shapes)

* `intake-document`: blob + metadata → `env.ctx` + `intent{ATTEST|BUNDLE}`
* `intake-event`: fato timestampado → `env.ctx` + `intent{EVAL|TRACE}`
* `intake-transaction`: duas partes + termos → `env.ctx` + `intent{EVAL|QUERY}`
  **Erro canônico:** `Err.Intake.ShapeViolation`, `Err.Intake.SchemaMismatch`.

### 2.2 Policy (5 famílias)

* `existence`: campos obrigatórios, schema, completude.
* `threshold`: métricas/limiares, SLA/quotas.
* `compliance`: regras regulatórias (EU-AI, HIPAA, FINRA, SOC2…).
* `provenance`: SBOM/SLSA/EER/assinaturas/linhagem.
* `authorization`: RBAC, escopos, grants, transações.
  **Saída:** `env.decision{ACK|NACK|ASK}`, `env.evidence{cids,urls}`, `env.metrics?`.
  **Erros:** `Err.Policy.MissingField`, `Err.Policy.RuleViolation`, `Err.Policy.Unverifiable`.

### 2.3 Verify

* `verify-capsule`: `id`, `seal`, `hdr.exp`, `ASCII DIDs`.
* `verify-chain`: `receipts.prev`, assinaturas hop, ordem.
* `verify-invariants`: ASK ⇒ `links.prev`; ACK/NACK ⇒ `evidence` (existente).
  **Erros:** `Err.Seal.BadSignature`, `Err.Hop.BadChain`, `Err.Hdr.Expired`, `Err.Inv.Invariant`.

### 2.4 Enrich

* `card_url`: resolve `rref` com content-negotiation (HTML/JSON).
* `badge`: SVG status; hash embarcado; bind a `capsule.id`.
* `status`: página HTML com decisão, evidências, cadeia.
* `pdf`: certificado (assinatura do emissor + QR com `capsule.id`).
* `ghost`: red/black mask (by field list), mantendo `rules_fired`.
* `webhook`: POST assinado (permit/tenant-scoped).
  **Erros:** `Err.Enrich.Render`, `Err.Webhook.Fail`, `Err.Ghost.MaskConfig`.

### 2.5 SIRP & Permit

* `sirp-capsule`: INTENT/RESULT + derive DELIVERY/EXEC receipts.
* `permit-issue/verify`: `ALLOW` ⇒ **assinatura do permit**, TTL por `hdr.exp`; `REQUIRE` ⇒ queue k-of-n.
  **Erros:** `Err.Permit.Scope`, `Err.Permit.Expired`, `Err.SIRP.ChainMismatch`.

---

## 3) Lifecycle: load → run → observe

1. **Load**: registrar módulos (feature flags **ou** WASM).
2. **Plan**: construir pipeline pelo **manifesto do produto** (ou por rota).
3. **Run**: `intake → evaluate → (require?) → permit → enrich → sirp → verify`
4. **Observe**: métricas + tracing por `capsule.id`; logs estruturados.

**Determinismo:** módulos **não** acessam clock/rede/FS; inputs só via env.

---

## 4) Config & Manifesto (módulos)

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

---

## 5) LLM-Engine & LLM-Smart — onde ajudam

* **LLM-Engine** (TDLN-first):

  * compila prompts/policies → grafos determinísticos;
  * faz *explanations* canônicas (`env.decision.reason`) sem “alucinação”;
  * **cache** por `capsule.id` + `rules_fired` (idempotência).
* **LLM-Smart** (mix premium/OSS):

  * roteamento **custo×latência×qualidade** por política;
  * compressão de `ctx` (schema-aware) p/ limites;
  * **fallbacks verificados** (se divergir, vira ASK + PoI).
* **Ambos** operam **apenas** sobre `env` (não tocam `hdr` / cadeia); tudo assinado no final via BASE.

---

## 6) Segurança & isolamentos

* WASM sandbox: **mem cap**, **instr cap**, **syscall deny** (exceto host shims).
* Secrets **nunca** em env; só referências (CIDs) + vault host-side.
* Permit obrigatório p/ qualquer side-effect (enrich/webhook/exec).
* **K-of-N** implementado host-side (fila idempotente; receipts de consentimento).

---

## 7) Observabilidade (por módulo)

* Métricas padrão: `duration_ms`, `error_total`, `ask_total`, `nack_total`.
* Tracing: `span: module_name@version`, `capsule_id`, `tenant`, `ulid`.
* Amostragem: 100% para `NACK/ASK`, 10% para `ACK`.

---

## 8) Testes & CI (p/ módulos)

### 8.1 Unit

* Esquemas/params; caminhos felizes e erros (`Err.*` padronizados).

### 8.2 Propriedade

* Idempotência de `evaluate`: re-run com mesmo input ⇒ mesma saída.
* `intake-*` gera mesma `env` indep. da ordem de campos de entrada.

### 8.3 Integração (pipeline)

* `intake → policy(existence,threshold) → permit(ALLOW) → enrich(webhook) → sirp → verify`.
* `ASK` com `links.prev` e PoI; `NACK` com `evidence` obrigatória.

### 8.4 Contratos

* Fuzz em `from_json` (ai-json-nrf1) e validadores.
* Snapshot de **ghost** (mascaramento) sem dados sensíveis.

**CI gates** (exemplo):

* `cargo clippy -D warnings`, `cargo test --workspace`
* `cargo fuzz run json_view_decode` (10m)
* Cobertura mínima p/ critical paths ≥ 75%.

---

## 9) Versionamento & compatibilidade

* Semântica **Mó[dulo@Major.Minor.Patch](mailto:dulo@Major.Minor.Patch)**.
* Quebra de contrato ⇒ **Major** (novo nome lógico).
* Manifesto de produto referencia módulos com ranges seguros: `"policy-existence@^1.2"`.

---

## 10) Erros padronizados (subset)

* `Err.Intake.ShapeViolation`, `Err.Intake.SchemaMismatch`
* `Err.Policy.RuleViolation`, `Err.Policy.MissingField`, `Err.Policy.Unverifiable`
* `Err.Permit.Scope`, `Err.Permit.Expired`, `Err.Permit.Denied`
* `Err.SIRP.ChainMismatch`, `Err.SIRP.BadSignature`
* `Err.Enrich.Render`, `Err.Webhook.Fail`, `Err.Ghost.MaskConfig`
* `Err.Inv.Invariant` (ASK/ACK/NACK invariantes quebrados)

---

## 11) DoD (Definition of Done) — MÓDULOS

* [ ] Runtime de Módulos (host) com: load, plan, run, WBE/HAL, permit/k-of-n.
* [ ] `intake-*` (3 shapes) + `policy-existence`, `policy-threshold` + `verify-standard`.
* [ ] `enrich-card_url`, `enrich-ghost`, `enrich-webhook`.
* [ ] `sirp-standard` + `permit-basic`.
* [ ] CI verde (unit/prop/integration/fuzz).
* [ ] Docs + exemplos com **capsulas** reais (ACK/ASK/NACK) e **cadeia de receipts**.

---

## 12) Exemplos rápidos

### 12.1 ASK por falta de evidência

* `policy-existence` detecta campo ausente → `decision=ASK` e `links.prev=<cid_prev>`; `enrich-ghost` exibe PoI sem PII.

### 12.2 SLA breach (`threshold`)

* p99 > 250ms ⇒ `NACK` + `evidence.urls=[status_page]`; `webhook` dispara para billing-proofs.

### 12.3 FINRA (domínio)

* `intake-transaction` + `policy-compliance/finra@1` + `permit-basic(k=2,n=3)`; require aprovações (com receipts de consent) antes de `ALLOW`.

---

## 13) Roadmap curto (para nave acender)

1. **Host** (runtime de módulos) + `intake-*` + `existence` + `threshold`.
2. **permit-basic** + `sirp-standard` + `verify-standard`.
3. **enrich-card_url/ghost/webhook** + exemplos end-to-end.
4. **LLM-engine/smart** plugados no `policy-*` como *advisors* determinísticos.

---


