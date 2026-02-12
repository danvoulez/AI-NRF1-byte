#3 — PRODUTOS (Do zero ao “rodando em cliente”)

> **Missão:** cada produto é um **binário** (ou container) que:
>
> 1. Recebe requisições HTTP/GRPC,
> 2. Orquestra **módulos** via manifesto,
> 3. Emite **UBL Capsule v1** (ai-nrf1) com **permit** + **SIRP**,
> 4. Publica **enrichments** (card URL, badge, ghost, webhook, PDF),
> 5. Entrega **bundle offline verificável**.

## 0) Anatomia de um produto

* **Processo**: `productd` (único binário por produto).
* **Entradas**: HTTP/GRPC; JSON view (ai-json-nrf1) com validação forte.
* **Motor**: runtime de módulos (Gigante #2) com pipeline declarado.
* **Saídas**:

  * Capsule (ai-nrf1) + **SIRP INTENT/RESULT** (+ DELIVERY/EXEC hops),
  * **Permit** para qualquer efeito,
  * **Card URL** (status JSON/HTML),
  * **Offline bundle** (ZIP) opcional.

## 1) Manifesto do produto (deploy-time)

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

## 2) API do produto (HTTP)

Rotas padrão (toda família de produtos):

* `POST /run` — entrada semântica (view JSON).
  **Efeito**: intake→policies→permit→sirp→verify→enrich; retorna **capsule.id** e **card_url**.
* `GET /r/{cid}` — **Card URL** com content-negotiation (HTML/JSON).
* `POST /verify` — recebe **bundle** ou capsule; devolve veredito + PoI.
* `POST /webhook/test` — valida assinatura e formato de callback.
* `GET /healthz` / `GET /readyz` — probes.
* `GET /.well-known/product.json` — manifesto servindo de SBOM funcional.

**Contrato de entrada**: `ai-json-nrf1` (view); o produto converte para **ai-nrf1** antes dos módulos.

## 3) Execução (caminho quente)

1. **Auth** (bearer/mtls) → `tenant`, `aud`.
2. **Parse** (view→ai-nrf1) + validação (NFC, ASCII, tipos).
3. **Pipeline**:

   * `intake-*` define `intent` e normaliza `ctx`;
   * `policy-*` decide `ACK/NACK/ASK` (+ `evidence`,`metrics`);
   * `permit`:

     * `ALLOW` ⇒ assina permit (escopo, exp) e **grava WBE**;
     * `REQUIRE` ⇒ abre fila k-of-n (se configurado) e retorna **ASK** com `links.prev`;
   * `sirp`: emite **INTENT/RESULT**, e **DELIVERY/EXEC** quando aplicável;
   * `verify`: reforça invariantes (id/selos/chain);
   * `enrich`: publica card/badge/ghost/webhook;
4. **Persistência**: NDJSON append-only por ULID + **content-addressed** (cid).
5. **Resposta**: `{cid, verdict, card_url, bundle_url?}`.

## 4) Permit + SIRP (sem brechas)

* **Permit**: objeto assinado (Ed25519/Dilithium) vinculando **exatamente** este input + expiração + escopo. Executor recusa sem permit válido.
* **SIRP**:

  * INTENT/RESULT sempre;
  * DELIVERY (destinatário assinando recebimento) e EXECUTION (executor assinando conclusão) conforme produto;
  * Cadeia encadeada (`prev`) com `receipt_id = blake3(payload_sem_sig)`;
  * **Offline bundle** inclui: capsule, receipts, chaves públicas, políticas, hashes, QR.

## 5) Observabilidade e SLO

* **Métricas**:

  * Latências: `p50/p95/p99` run; distribuição por módulo.
  * Contagem: `ack_total`, `nack_total`, `ask_total`, `replay_blocked`, `permit_issued`.
  * Webhook: `attempts`, `success`, `fail`, `retry`.
* **Tracing**: `capsule.id`, `tenant`, `route`, `module@ver`, `decision`.
* **Logs**: estruturados, sem PII (ghost e mask lists).
* **SLO exemplo** (Gateway): p95 < 150ms; erro < 0.1%; disponibilidade 99.9%.

## 6) Segurança

* **WBE/HAL** fortemente aplicado (antes de side-effects).
* **Secrets** apenas no host; módulos recebem **CIDs**.
* **Replay** bloqueado via `(hdr.src, hdr.nonce)` + cache; `exp` auditável (sem TTL mutável).
* **Consent k-of-n** host-side; recibos de consentimento entram na cadeia.
* **Policy sandbox** (WASM): sem clock/rede/FS; tempo/mem caps.

## 7) Testes (DoD por produto)

* **Unit**: validação de entrada; mapping view↔ai-nrf1; erros `Err.*`.
* **Integração**:

  * **ACK happy-path**: run → permit → sirp → verify → enrich → card_url.
  * **ASK**: falta evidência ⇒ `links.prev`, fila k-of-n, promoção a `ALLOW`.
  * **NACK**: regra violada ⇒ evidence obrigatória; badge vermelho.
  * **Bundle**: `verify-bundle` reconstruindo cadeia com chaves/algoritmos.
* **Propriedade**: idempotência (mesmo input ⇒ mesmo `cid`); determinismo (sem I/O).
* **Fuzz**: json_view parsers; unicode NFC; varint minimal.
* **Perf**: throughput alvo (ex.: 2k RPS p/ Gateway) com p95 < SLO.

## 8) Deploy

* **Container** com:

  * Binário `productd`,
  * `product.json` (manifesto),
  * `modules.lock` (resolução exata),
  * chaves de selo/permit (via KMS/Secrets).
* **Ambientes**: `dev`, `stage`, `prod` (chaves/URLs distintas).
* **Imutabilidade**: tags por `cid` do manifesto; rollback = trocar tag.
* **Compat**: wire protocol estável; breaking ⇒ nova major do produto.

---

## 9) Catálogo — presets prontos

### 9.1 API Receipt Gateway (P0)

* **Act**: TRANSACT (event).
* **Policies**: existence + authorization/gateway.
* **Proof**: **SIRP** (INTENT/RESULT + DELIVERY/EXEC).
* **Enrich**: card_url, ghost, webhook.
* **Billing**: `per_receipted_event`.
* **SLO**: p95 < 150ms.
* **Rotas**: `POST /run`, `GET /r/{cid}`, `POST /verify`.

### 9.2 AI Model Passport (P1)

* **Acts**: ATTEST → EVALUATE.
* **Policies**: existence + compliance/EU-AI + threshold de benchmarks.
* **Proof**: **bundle**.
* **Enrich**: badge, pdf, status.
* **Billing**: `per_evaluation`.

### 9.3 Notarize-and-Prove (P1)

* **Act**: ATTEST.
* **Policies**: existence + provenance.
* **Proof**: **bundle**.
* **Enrich**: pdf, status.
* **Billing**: `per_evaluation`.

*(Demais: Compliance Receipts, Metered Billing, Supply Chain, Proof-for-Access, SLA Enforcement — seguem o mesmo molde com trocas em `intake`/policies/enrich/proof.)*

---

## 10) Exemplo end-to-end (Gateway)

1. `POST /run` com:

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

2. Produto → intake/event → policy/authorization → **ALLOW**.
3. Host emite **permit**, grava WBE, executa enrich, **SIRP** INTENT/RESULT/DELIVERY/EXEC.
4. Retorno:

```json
{ "cid":"b3:...", "verdict":"ACK", "card_url":"https://.../r/b3:..." }
```

5. `GET /r/b3:...` (HTML/JSON) — mostra decisão e cadeia; baixar **bundle**.
6. `POST /verify` com bundle — `OK`, PoI vazio.

---

## 11) Definition of Done — PRODUTO

* [ ] Manifesto resolvido e imutável (`cid` do product.json).
* [ ] API exposta (`/run`, `/r/{cid}`, `/verify`, probes).
* [ ] Pipeline implementado com módulos declarados (intake/policy/permit/sirp/verify/enrich).
* [ ] **Permit** verificado pelo executor (nenhuma ação sem permit).
* [ ] **SIRP** com cadeia íntegra; **bundle** verificável offline.
* [ ] **Card URL** com negotiation (HTML/JSON); webhook assinado.
* [ ] CI verde (unit/integration/property/fuzz/perf smoke).
* [ ] Observabilidade e SLOs instrumentados.
* [ ] Playbook de deploy/rollback (tags por `cid`).

---

## 12) Próximos passos práticos

1. **Escolher P0**: **API Receipt Gateway** (confirmado).
2. Gerar repositório `product-api-receipt-gateway` com scaffold:

   * `/cmd/productd`, `/configs/product.json`, `/modules.lock`, `/charts/helm`.
3. Integrar **intake-event@1**, **policy-authorization/gateway@1**, **permit-basic@1**, **sirp-standard@1**, **enrich-card_url/webhook/ghost**, **verify-standard@1**.
4. Subir **/run** + **/r/{cid}** + **/verify**; escrever 6 testes de integração (ACK/ASK/NACK, bundle, webhook, replay).
5. Medir p95; ajustar limites; congelar 1.0.0.


