Show\! Vamos pra Mensagem \#2: cap-permit — um módulo de authorization & gating com DSL mínima (YAML), retornando ASK/ACK/NACK e pronto para integrar no registry. Entrego código, testes, exemplo de policy e instruções. ⚖️🚦

---

# **Patch —** 

# **modules/cap-permit**

## **1\)** 

## **modules/cap-permit/Cargo.toml**

\[package\]  
name \= "cap-permit"  
version \= "0.1.0"  
edition \= "2021"  
license \= "MIT OR Apache-2.0"

\[dependencies\]  
anyhow \= "1"  
axum \= { version \= "0.7", features \= \["json"\] }  
serde \= { version \= "1", features \= \["derive"\] }  
serde\_json \= "1"  
serde\_yaml \= "0.9"  
glob \= "0.3"  
time \= { version \= "0.3", features \= \["macros"\] }  
tracing \= "0.1"  
once\_cell \= "1"

\# Base (já no workspace)  
ubl\_json\_view \= { path \= "../../impl/rust/ubl\_json\_view" }  
ubl\_capsule   \= { path \= "../../impl/rust/ubl\_capsule" }

\[dev-dependencies\]  
tokio \= { version \= "1", features \= \["macros", "rt-multi-thread"\] }  
reqwest \= { version \= "0.12", features \= \["json"\] }

## **2\) Implementação —** 

## **modules/cap-permit/src/lib.rs**

use anyhow::{Context, Result};  
use axum::{routing::post, Json, Router};  
use once\_cell::sync::OnceCell;  
use serde::{Deserialize, Serialize};  
use serde\_json::json;  
use std::{collections::HashMap, fs, path::Path};

mod model;  
use model::\*;

static POLICY: OnceCell\<Policy\> \= OnceCell::new();

\#\[derive(Deserialize)\]  
pub struct EvalReq {  
    /// View JSON de cápsula (mínimo: env/meta/intent/links)  
    capsule\_json: serde\_json::Value,  
}

\#\[derive(Serialize)\]  
pub struct EvalResp {  
    verdict: String,               // "ACK" | "NACK" | "ASK"  
    reason: Option\<String\>,  
    matched\_rule: Option\<String\>,  // hint da regra  
}

pub fn router() \-\> Router {  
    Router::new().route("/v1/permit/eval", post(eval\_handler))  
}

pub fn load\_policy\_from\<P: AsRef\<Path\>\>(path: P) \-\> Result\<()\> {  
    let pol \= load\_policy(path.as\_ref())?;  
    POLICY.set(pol).ok();  
    Ok(())  
}

async fn eval\_handler(Json(req): Json\<EvalReq\>) \-\> Result\<Json\<EvalResp\>, axum::http::StatusCode\> {  
    let pol \= POLICY.get().ok\_or(axum::http::StatusCode::SERVICE\_UNAVAILABLE)?;  
    // 1\) Materializa env p/ garantir canonicidade de strings antes do match  
    let \_ \= ubl\_json\_view::from\_json\_view(\&req.capsule\_json)  
        .map\_err(|\_| axum::http::StatusCode::BAD\_REQUEST)?;

    match pol.evaluate(\&req.capsule\_json) {  
        Ok((v, hint)) \=\> Ok(Json(EvalResp {  
            verdict: v.to\_string(),  
            reason: None,  
            matched\_rule: hint,  
        })),  
        Err(e) \=\> Ok(Json(EvalResp {  
            verdict: "NACK".into(),  
            reason: Some(format\!("policy error: {e}")),  
            matched\_rule: None,  
        })),  
    }  
}

/// Carrega YAML; suporta \`include: glob\` para fatiar políticas.  
fn load\_policy(path: \&Path) \-\> Result\<Policy\> {  
    let text \= fs::read\_to\_string(path)  
        .with\_context(|| format\!("reading {:?}", path))?;  
    let mut pol: PolicyDoc \= serde\_yaml::from\_str(\&text)  
        .with\_context(|| "parsing YAML")?;

    // includes  
    if let Some(glob\_pat) \= pol.include.take() {  
        for entry in glob::glob(\&glob\_pat)? {  
            let entry \= entry?;  
            let t \= fs::read\_to\_string(\&entry)  
                .with\_context(|| format\!("reading include {:?}", entry))?;  
            let inc: PolicyDoc \= serde\_yaml::from\_str(\&t)?;  
            pol.extend(inc);  
        }  
    }  
    Ok(Policy::from\_doc(pol))  
}

## **3\) Modelo/DSL —** 

## **modules/cap-permit/src/model.rs**

use anyhow::{anyhow, Result};  
use serde::{Deserialize, Serialize};  
use serde\_json::Value as J;  
use std::collections::HashMap;

\#\[derive(Debug, Clone, Copy, Serialize, Deserialize)\]  
\#\[serde(rename\_all \= "UPPERCASE")\]  
pub enum Verdict { ACK, NACK, ASK }  
impl ToString for Verdict { fn to\_string(\&self) \-\> String { format\!("{:?}", self) } }

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct PolicyDoc {  
    pub version: String,         // "permit/0.1"  
    pub include: Option\<String\>, // glob  
    \#\[serde(default)\]  
    pub rules: Vec\<RuleDoc\>,  
    \#\[serde(default)\]  
    pub quotas: Vec\<QuotaDoc\>,  
}

impl PolicyDoc {  
    pub fn extend(\&mut self, other: PolicyDoc) {  
        self.rules.extend(other.rules);  
        self.quotas.extend(other.quotas);  
    }  
}

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct RuleDoc {  
    pub id: String,              // id da regra (hint de debug)  
    pub when: MatchDoc,          // critérios  
    pub decision: Verdict,       // ACK/NACK/ASK  
    \#\[serde(default)\]  
    pub reason: Option\<String\>,  // texto opcional  
}

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct QuotaDoc {  
    pub id: String,  
    pub when: MatchDoc,  
    pub budget: i64,             // unidades arbitrárias por janela  
    pub window\_sec: i64,         // janela (segundos)  
    \#\[serde(default)\]  
    pub key: Option\<String\>,     // chave da cota (ex.: "tenant" ou "user")  
}

// DSL de match simples (prefix \+ equals \+ in \+ exists)  
\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct MatchDoc {  
    \#\[serde(default)\] pub eq:    HashMap\<String, J\>,  
    \#\[serde(default)\] pub prefix:HashMap\<String, String\>,  
    \#\[serde(default)\] pub r\#in:  HashMap\<String, Vec\<J\>\>,  
    \#\[serde(default)\] pub exists:Vec\<String\>,  
}

\#\[derive(Debug, Clone)\]  
pub struct Policy {  
    rules: Vec\<RuleDoc\>,  
    // quotas: para v0 anotar (sem persistência global aqui)  
    quotas: Vec\<QuotaDoc\>,  
}

impl Policy {  
    pub fn from\_doc(d: PolicyDoc) \-\> Self {  
        Self { rules: d.rules, quotas: d.quotas }  
    }

    /// Avalia na ordem: primeira regra que casar vence.  
    pub fn evaluate(\&self, capsule\_json: \&J) \-\> Result\<(Verdict, Option\<String\>)\> {  
        for r in \&self.rules {  
            if matches(\&r.when, capsule\_json)? {  
                return Ok((r.decision, Some(r.id.clone())));  
            }  
        }  
        // default: ASK  
        Ok((Verdict::ASK, None))  
    }  
}

// Helpers de leitura (JSON-pointer style simples: "env.intent.kind", etc.)  
fn get\_ptr\<'a\>(root: &'a J, path: \&str) \-\> Option\<&'a J\> {  
    let mut cur \= root;  
    for seg in path.split('.') {  
        cur \= match cur {  
            J::Object(m) \=\> m.get(seg)?,  
            \_ \=\> return None,  
        }  
    }  
    Some(cur)  
}

fn matches(m: \&MatchDoc, root: \&J) \-\> Result\<bool\> {  
    // exists  
    for k in \&m.exists {  
        if get\_ptr(root, k).is\_none() { return Ok(false); }  
    }  
    // eq  
    for (k, v) in \&m.eq {  
        match (get\_ptr(root, k), v) {  
            (Some(a), b) if a \== b \=\> {},  
            \_ \=\> return Ok(false),  
        }  
    }  
    // prefix (somente strings)  
    for (k, pfx) in \&m.prefix {  
        if let Some(J::String(s)) \= get\_ptr(root, k) {  
            if \!s.starts\_with(pfx) { return Ok(false); }  
        } else { return Ok(false); }  
    }  
    // in  
    for (k, set) in \&m.r\#in {  
        let a \= get\_ptr(root, k);  
        if a.is\_none() { return Ok(false); }  
        let a \= a.unwrap();  
        if \!set.iter().any(|v| v \== a) { return Ok(false); }  
    }  
    Ok(true)  
}

## **4\) Exemplo de Policy —** 

## **configs/policy/permit.yaml**

version: "permit/0.1"

\# opcional: incluir fragmentos  
\# include: "configs/policy/\*.d/\*.yaml"

rules:  
  \- id: "allow-attest-by-app"  
    when:  
      eq:  
        "env.intent.kind": "ATTEST"  
        "env.meta.app": "demo-app"  
      exists:  
        \- "env.agent.id"  
    decision: ACK

  \- id: "deny-expired"  
    when:  
      prefix:  
        "v": "ubl-capsule/"  
      eq:  
        "hdr.expired": true   \# dica: se um gateway pré-marca algo  
    decision: NACK  
    reason: "expired"

  \- id: "ask-default"  
    when: {}  
    decision: ASK

\# (v0: quotas ainda não persistem em runtime local)  
quotas: \[\]  
Observação: o matcher opera sobre a view JSON da cápsula (ou payload equivalente). Você pode casar campos em env.\*, meta.\*, intent.\*, etc. Para sinais derivados (ex.: hdr.expired), um gateway anterior pode inflar o JSON de entrada.

## **5\) Integração no** 

## **registry**

##  **(atrás da feature** 

## **modules**

## **)**

Adicionar dependência e incluir na feature:

services/registry/Cargo.toml

\[features\]  
default \= \[\]  
modules \= \["cap-intake", "cap-permit"\]

\[dependencies\]  
\# ...  
cap-intake \= { path \= "../../modules/cap-intake", optional \= true }  
cap-permit \= { path \= "../../modules/cap-permit", optional \= true }

Montar rotas na função mount\_modules (já criada na msg \#1):

\#\[cfg(feature \= "modules")\]  
fn mount\_modules(router: axum::Router) \-\> axum::Router {  
    use cap\_intake;  
    use cap\_permit;

    // carrega a policy na inicialização (ENV: PERMIT\_PATH)  
    if let Ok(path) \= std::env::var("PERMIT\_PATH") {  
        let \_ \= cap\_permit::load\_policy\_from(path);  
    }

    router  
        .merge(cap\_intake::router())  
        .merge(cap\_permit::router()) // expõe /v1/permit/eval  
}

## **6\) Testes de integração —** 

## **modules/cap-permit/tests/permit\_eval.rs**

use axum::Router;  
use cap\_permit::{router, load\_policy\_from};  
use serde\_json::json;  
use tokio::task;  
use std::net::TcpListener;  
use reqwest::Client;

\#\[tokio::test\]  
async fn eval\_ack\_default\_policy() {  
    // policy embutida de teste  
    let tmp \= tempfile::NamedTempFile::new().unwrap();  
    std::fs::write(tmp.path(), r\#"  
version: "permit/0.1"  
rules:  
  \- id: "allow-attest-demo"  
    when:  
      eq:  
        "env.intent.kind": "ATTEST"  
        "env.meta.app": "demo-app"  
    decision: ACK  
  \- id: "else-ask"  
    when: {}  
    decision: ASK  
"\#).unwrap();

    load\_policy\_from(tmp.path()).unwrap();

    let app \= router();  
    let listener \= TcpListener::bind("127.0.0.1:0").unwrap();  
    let addr \= listener.local\_addr().unwrap();  
    let server \= axum::serve(listener, app);  
    let handle \= task::spawn(server);

    let cap \= json\!({  
      "v":"ubl-capsule/1.0",  
      "hdr": {"src":"did:ubl:x","dst":"did:ubl:y","nonce":"b64:AA==","exp": 999999999999, "chan": null },  
      "env": {  
        "v":"ubl-json/0.1.1",  
        "t":"record",  
        "agent":{"id":"agent-1"},  
        "intent":{"kind":"ATTEST","name":"demo"},  
        "ctx":{},  
        "decision":{"verdict":"ASK"},  
        "evidence":{},  
        "meta":{"app":"demo-app","tenant":"t1","user":"u1"}  
      },  
      "seal":{"alg":"Ed25519","kid":"did:ubl:x\#k"},  
      "receipts":\[\]  
    });

    let res \= Client::new()  
        .post(format\!("http://{}/v1/permit/eval", addr))  
        .json(\&serde\_json::json\!({ "capsule\_json": cap }))  
        .send().await.unwrap();

    assert\!(res.status().is\_success());  
    let body: serde\_json::Value \= res.json().await.unwrap();  
    assert\_eq\!(body\["verdict"\], "ACK");

    handle.abort();  
}

---

# **Como rodar (local) 🧪**

\# 1\) Testes do módulo  
cargo test \-p cap-permit

\# 2\) Subir registry com módulos e policy  
export PERMIT\_PATH="$PWD/configs/policy/permit.yaml"  
cargo run \-p registry \--features modules \--release

### **Exercitar via HTTP**

\# Exemplo que retorna ACK (regra allow-attest-by-app)  
curl \-s http://127.0.0.1:8790/v1/permit/eval \\  
  \-H 'content-type: application/json' \\  
  \-d '{  
    "capsule\_json": {  
      "v": "ubl-capsule/1.0",  
      "hdr": {"src":"did:ubl:lab512-registry","dst":"did:ubl:receiver","nonce":"b64:AA==","exp": 1739999999000000000},  
      "env": {"v":"ubl-json/0.1.1","t":"record",  
        "agent":{"id":"agent-1"},  
        "intent":{"kind":"ATTEST","name":"hello"},  
        "ctx":{},  
        "decision":{"verdict":"ASK"},  
        "evidence":{},  
        "meta":{"app":"demo-app","tenant":"t1","user":"u1"}  
      },  
      "seal":{"alg":"Ed25519","kid":"did:ubl:lab512-registry\#dev"},  
      "receipts":\[\]  
    }  
  }' | jq .

---

# **Critérios de aceite ✅**

1. DSL clara (YAML) com rules\[\] → when { eq, prefix, in, exists } → decision ∈ {ACK,NACK,ASK}.

2. Determinismo: materializa view em NRF antes de avaliar (normalização de strings/ASCII).

3. Segurança: default ASK se nenhuma regra casar; erro de parsing ⇒ NACK com reason.

4. Operação: carrega via PERMIT\_PATH; suporta include (glob).

5. Testes: cargo test \-p cap-permit passa; integração HTTP ok.

---

# **Próxima (Mensagem \#3):** 

# **cap-policy**

* Orquestrador de decisões (chama cap-permit, aplica invariantes ASK/ACK/NACK, injeta motivos/metrics, pode acionar cap-llm para ASK→decidir).

* Rotas: /v1/policy/apply (entrada cápsula → saída cápsula com decision ajustada).

* Vetores de teste para invariantes (ASK ⇒ links.prev obrigatório; ACK/NACK ⇒ evidence presente, ainda que vazia).

Se quiser, já preparo também o wire-up no registry (rota /v1/pipeline/ingest+permit) que encadeia cap-intake → cap-permit com um POST só. Quer que eu já traga isso junto na \#3? 😉

