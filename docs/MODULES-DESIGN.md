# Module Architecture — Design Document

> **Source**: `MODULOS-INICIO.md` (iterative design session)
> **Date**: 2026-02
> **Status**: Reference — architectural rationale for the MODULE phase.
> **Code**: Extracted separately into `EXTRACTED/code/` for review before placement.

---


## Modelo Canônico de Módulo (Capability) — versão refinada
### 0) Premissas (invariantes do ecossistema)

1.  **Canon = bytes (ai-nrf1 / ubl-byte)**.  
    JSON/YAML = _view_, input humano, ou config. Nunca “verdade”.

2.  **Produto = pipeline declarativo** (manifesto), não um codebase novo.  
    Um “produto” só existe se eu conseguir descrevê-lo como “steps + configs + packs/assets por CID”.

3.  **Módulo é capacidade**, não caso de uso.  
    Ex: `cap-notify` (driver webhook/email/slack) e não `mod-slack-finra`.

4.  **Tudo que for “conteúdo” é asset content-addressed**: policy packs, templates, prompts, schemas, regras, etc.

---
### 1) O formato real do módulo (o que ele é no repo)
Um módulo é um **crate Rust** (biblioteca), com API estável, e implementação “core puro” separada de IO.

Estrutura recomendada (igual pra todos):

```text
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

---
### 2) O contrato do módulo (um shape só, sempre)
Pra evitar “cada módulo inventa um input/output”, a gente padroniza **um envelope universal**.

### `CapInput`

* `env`: ai-json-nrf1 (a semântica)
* `capsule`: opcional (se já está no nível UBL Capsule)
* `pipeline_prev`: lista de CIDs (composição de atos)
* `assets`: refs resolvidas (packs/templates/etc), **por CID**
* `meta`: metadados de execução (run\_id, tenant, trace\_id, etc)

### `CapOutput`

* `env`: (imutável ou “novo env”, mas sempre explícito)
* `artifacts`: html/svg/pdf/json (outputs derivados)
* `effects`: side-effects declarados (ex.: “enviar webhook X”)
* `receipts`: receipts/capsules gerados (quando aplicável)
* `metrics/events`: observabilidade e auditoria

O ponto: o módulo pode até ter structs internas próprias, mas **a borda é padronizada**.

---
### 3) Separar “resultado” de “efeito” (isso mata metade do caos)
O que costuma explodir projetos desse tipo é módulo fazendo IO “por baixo”.

Então: **capabilities devem preferir retornar “effects”** em vez de executar.

### Exemplo

* `cap-enrich` retorna `Artifact::HtmlStatusPage(bytes)` e `Effect::PublishToCardURL { cid, bytes }`
* o runtime (orquestrador) decide se publica, re-tenta, assina, enfileira etc.

Isso te dá:

* reprocessamento idempotente
* retries seguros
* execução offline e teste fácil
* “async/await gateway” vira detalhe do runtime, não do módulo

---
### 4) `Ctx` (contexto único), com “ports” bem definidos
Módulo não conversa com o mundo direto. Conversa via interfaces.

**Ports mínimos:**

* `Store`: get/put por CID (conteúdo e assets)
* `Clock`: now\_nanos()
* `Keyring`: sign/verify (Ed25519/PQ)
* `AuditSink`: append NDJSON (log à prova de auditoria)
* `Net`: (opcional) fetch externo (mas idealmente só o runtime usa)
* `Queue`: (opcional) para efeitos async

Na prática, `Ctx` vira o “hardware” da plataforma.

---
### 5) O que vai no `Config` (derivado do manifesto)
O config deve ser **declarativo** e **validável**.

Campos canônicos:

* `id`: nome lógico do step (ex.: `"policy_main"`)
* `kind`: `"cap-policy"`, `"cap-intake"`, etc
* `assets`: lista de `CidRef` (packs, templates, schema refs…)
* `params`: knobs do cap (strict, mode, timeouts, redaction etc)
* `io`: “declaração” de endpoints/drivers (sem segredo; segredo é via env/ctx)

E sempre: `validate()` que checa invariantes e resolve refs.

---
### 6) Versionamento (pra não quebrar produtos)
Cada cap deve declarar:

* `cap_kind`: `"cap-policy"`
* `cap_api_version`: `"1.0"`
* `compat`: lista de versões aceitas do manifesto/config

Regra: **produto aponta pra cap\_kind + cap\_api\_version**, não pra crate.

Assim você consegue:

* evoluir implementações
* manter compat de manifesto
* evitar refactor que quebra tudo

---
### 7) Idempotência e determinismo (a checklist obrigatória)
Cada cap deve declarar, explicitamente:

* **Determinístico?** (mesmo input + mesmos assets + mesma seed → mesmo output)
* **Idempotente?** (rodar duas vezes não duplica efeito / receipts)
* **Reentrante?** (pode retomar de uma falha sem drift)
* **Pure vs Effectful?** (ideal: puro e retorna effects)

E o runtime deve ter:

* `dedupe_key` por step (ex.: hash do `CapInput` canônico)
* cache opcional de outputs por `dedupe_key`

---
### 8) Logging / Auditoria (sem vazar segredo)
Módulo escreve:

* `AuditEvent` com hash do input canônico + resumo
* nunca escreve payload bruto sensível (só CID + redaction)

Padrão:

* `event.kind`, `event.step_id`, `event.run_id`, `event.input_cid`, `event.output_cid`, `event.verdict` (se existir)

Isso faz teu “ops/audit” virar matemática.

---
### 9) Taxonomia mínima de capabilities (pra evitar 200 módulos)
Eu refinaria teus 5 como **famílias**, e não módulos únicos:

### A) `cap-intake` (Normalize)

* entra “mundo” → sai env canônico
* mapping spec (jsonpath/jq-like) + schema check
* pode usar `cap-llm` como auxiliar (mas output vira dado)

### B) `cap-policy` (Decide)

* env + packs → decision (ACK/NACK/ASK + PoI)
* packs podem ser wasm/json (mas “pack” é asset por CID)

### C) `cap-permit` (Consent)

* transforma ASK/REQUIRE em ticket→permit
* K-of-N, TTL por exp, timeout\_action

### D) `cap-enrich` (Render)

* receipt/decision → artifacts/effects
* drivers: html, badge, pdf, webhook, ghost
* redaction config por manifesto

### E) `cap-transport` (SIRP/Relay)

* lida com hdr/nonce/exp/receipts hop-chain
* é a ponte pra “mundo rede”

### F) `cap-llm` (Assist)

* interface única “ask model”
* output sempre “proposta”, nunca decisão final
* guarda prompt/template como asset por CID

Percebe? Eu **não colocaria transport dentro de enrich** e nem “LLM assist” solto: ambos merecem família própria.

---
### 10) Runtime/Orquestrador (onde entra o “gateway async/await”)
Isso é importante: **a bagunça não é resolvida dentro de módulos**. É resolvida no runtime.

O runtime é quem:

* executa pipeline steps em ordem
* resolve assets por CID
* aplica dedupe/idempotência
* executa effects (sync ou async)
* mantém state machine (NEW → READY → RUNNING → DONE/ERROR)
* emite a UBL Capsule e hop receipts se o produto pede “proof\_level: sirp/bundle”

Então: **sim**, é comum precisar de um “gateway async/await” — mas ele é o **runtime executor**, não um módulo ad hoc.

---
### 11) “Core module vs general module” (sem complicar demais)
Você não precisa dessa divisão formal agora. Mas dá pra usar um critério simples:

* **Core caps**: necessários pra qualquer produto (intake, policy, enrich, transport, permit)
* **Addon caps**: integrações específicas (ex.: driver “smtp”, “slack api”, “s3 store”) — mas isso deve entrar como **drivers** dentro de famílias core, não como cap novo.

Ou seja: _não_ “cap-slack”; sim “cap-enrich(driver=slack)” ou “cap-notify(driver=slack)”.

---
## Minha versão (resumida) do teu plano
O teu plano é bom. Minha “versão plataforma” fica:

1.  **cap-intake** (normalize)
2.  **cap-policy** (decide)
3.  **cap-permit** (human gate)
4.  **cap-enrich** (render + effects)
5.  **cap-transport** (SIRP + hop receipts)
6.  **cap-llm** (assist confinado)

E um **runtime** forte que:

* compõe por manifesto
* executa effects async
* garante dedupe/idempotência
* controla state machine e audit

---
### 12) Regras de ouro do módulo (anti-caos)

1.  **Determinismo por default**: qualquer coisa dependente de tempo/aleatório entra via `Ctx`.
2.  **Sem parsing “solto”**: view JSON vira canonical antes de entrar no core.
3.  **Sem decisões em LLM**: LLM só gera _dados_, nunca verdict.
4.  **Sem módulo por caso de uso**: caso de uso = `Config + Packs + Templates`.
5.  **Tudo que é asset é CID**: prompt/template/policypack = content addressed.
