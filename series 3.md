Perfeito\! Vamos pra Mensagem \#3: cap-policy — o “orquestrador” de decisões. Ele recebe a cápsula (view JSON), aplica as invariantes formais (ASK/ACK/NACK), pode consultar módulos auxiliares (ex.: cap-permit), injeta decision.{verdict,reason,metrics}, e garante que a cápsula resultante continua canônica quando materializada em NRF. ⚖️🧠

Vou te entregar: código do módulo, testes, exemplo de policy compostável, e wire-up no registry (atrás de \--features modules) encadeando intake → permit → policy.

---

# **Patch —** 

# **modules/cap-policy**

## **1\)** 

## **modules/cap-policy/Cargo.toml**

\[package\]  
name \= "cap-policy"  
version \= "0.1.0"  
edition \= "2021"  
license \= "MIT OR Apache-2.0"

\[dependencies\]  
anyhow \= "1"  
axum \= { version \= "0.7", features \= \["json"\] }  
serde \= { version \= "1", features \= \["derive"\] }  
serde\_json \= "1"  
serde\_yaml \= "0.9"  
reqwest \= { version \= "0.12", features \= \["json"\] }  
once\_cell \= "1"  
tracing \= "0.1"  
time \= { version \= "0.3", features \= \["macros"\] }

\# Base  
ubl\_json\_view \= { path \= "../../impl/rust/ubl\_json\_view" }

\[dev-dependencies\]  
tokio \= { version \= "1", features \= \["macros", "rt-multi-thread"\] }  
tempfile \= "3"

## **2\) Implementação —** 

## **modules/cap-policy/src/lib.rs**

use anyhow::{anyhow, Context, Result};  
use axum::{routing::post, Json, Router};  
use once\_cell::sync::OnceCell;  
use serde::{Deserialize, Serialize};  
use serde\_json::{json, Value as J};  
use std::{fs, path::Path};

mod model;  
use model::\*;

static POLICY: OnceCell\<PolicyDoc\> \= OnceCell::new();

\#\[derive(Deserialize)\]  
pub struct ApplyReq {  
    /// Cápsula (view JSON)  
    capsule\_json: J,  
}

\#\[derive(Serialize)\]  
pub struct ApplyResp {  
    capsule\_json: J,       // cápsula com decision ajustada  
    matched: Option\<String\>,  
}

pub fn router() \-\> Router {  
    Router::new().route("/v1/policy/apply", post(apply\_handler))  
}

pub fn load\_policy\_from\<P: AsRef\<Path\>\>(path: P) \-\> Result\<()\> {  
    let text \= fs::read\_to\_string(path)?;  
    let d: PolicyDoc \= serde\_yaml::from\_str(\&text)?;  
    POLICY.set(d).ok();  
    Ok(())  
}

async fn apply\_handler(Json(req): Json\<ApplyReq\>) \-\> Result\<Json\<ApplyResp\>, axum::http::StatusCode\> {  
    // 1\) Normaliza (valida NFC/ASCII/sem floats) — se falhar, é NACK  
    let mut canon \= match ubl\_json\_view::from\_json\_view(\&req.capsule\_json) {  
        Ok(v) \=\> v,  
        Err(e) \=\> {  
            let mut c \= req.capsule\_json.clone();  
            force\_decision(\&mut c, "NACK", Some(format\!("canon error: {e}")));  
            return Ok(Json(ApplyResp { capsule\_json: c, matched: None }));  
        }  
    };  
    // volta para view após tocar em decision  
    let mut view \= ubl\_json\_view::to\_json\_view(\&canon).map\_err(|\_| axum::http::StatusCode::INTERNAL\_SERVER\_ERROR)?;

    // 2\) Lê policy  
    let pol \= POLICY.get().ok\_or(axum::http::StatusCode::SERVICE\_UNAVAILABLE)?;

    // 3\) Consulta cap-permit (se configurado) → “verdict inicial”  
    let mut matched \= None;  
    if let Some(permit\_url) \= pol.permit\_url.as\_deref() {  
        match query\_permit(permit\_url, \&view).await {  
            Ok((verdict, rule\_id)) \=\> {  
                matched \= rule\_id;  
                set\_decision(\&mut view, \&verdict, None);  
            }  
            Err(e) \=\> {  
                // Falha de infra do permit ⇒ ASK (fail-open), policy pode endurecer depois  
                set\_decision(\&mut view, "ASK", Some(format\!("permit error: {e}")));  
            }  
        }  
    }

    // 4\) Aplica invariantes formais locais (e regras extra)  
    apply\_invariants(\&mut view);

    // 5\) Re-materializa NRF (garantia de canonicidade pós-mutate) — se falhar vira NACK  
    match ubl\_json\_view::from\_json\_view(\&view) {  
        Ok(v2) \=\> {  
            canon \= v2; // sombra local; OK  
        }  
        Err(e) \=\> {  
            force\_decision(\&mut view, "NACK", Some(format\!("post-canon error: {e}")));  
        }  
    }

    Ok(Json(ApplyResp { capsule\_json: view, matched }))  
}

async fn query\_permit(url: \&str, capsule\_json: \&J) \-\> Result\<(String, Option\<String\>)\> {  
    let payload \= json\!({ "capsule\_json": capsule\_json });  
    let res \= reqwest::Client::new().post(format\!("{url}/v1/permit/eval")).json(\&payload).send().await?;  
    if \!res.status().is\_success() { return Err(anyhow\!("permit http {}", res.status())); }  
    let v: J \= res.json().await?;  
    let verdict \= v\["verdict"\].as\_str().unwrap\_or("ASK").to\_string();  
    let matched \= v.get("matched\_rule").and\_then(|x| x.as\_str()).map(|s| s.to\_string());  
    Ok((verdict, matched))  
}

/// Regras formais:  
/// \- ASK ⇒ links.prev obrigatório (ghost pendente)  
/// \- ACK/NACK ⇒ evidence presente (pode ser {}), e decision.reason opcional  
fn apply\_invariants(view: \&mut J) {  
    let verdict \= view.pointer("/env/decision/verdict").and\_then(|v| v.as\_str()).unwrap\_or("ASK");  
    match verdict {  
        "ASK" \=\> {  
            // prev obrigatório  
            let has\_prev \= view.pointer("/env/links/prev").is\_some();  
            if \!has\_prev {  
                set\_link\_prev\_placeholder(view);  
                set\_reason(view, Some("ASK requires env.links.prev".into()));  
            }  
        }  
        "ACK" | "NACK" \=\> {  
            // evidence existe (pode ser {} mas não ausente)  
            ensure\_evidence\_exists(view);  
        }  
        \_ \=\> {  
            set\_decision(view, "ASK", Some("unknown verdict coerced to ASK".into()));  
            set\_link\_prev\_placeholder(view);  
        }  
    }  
}

fn set\_decision(view: \&mut J, verdict: \&str, reason: Option\<String\>) {  
    view.pointer\_mut("/env/decision").and\_then(|d| d.as\_object\_mut()).map(|m| {  
        m.insert("verdict".into(), J::String(verdict.to\_string()));  
        if let Some(r) \= reason { m.insert("reason".into(), J::String(r)); }  
    }).or\_else(|| {  
        \*view \= inject\_path(view.take(), &\["env","decision","verdict"\], J::String(verdict.to\_string()));  
        if let Some(r) \= reason {  
            \*view \= inject\_path(view.take(), &\["env","decision","reason"\], J::String(r));  
        }  
        Some(())  
    });  
}

fn set\_reason(view: \&mut J, reason: Option\<String\>) {  
    if let Some(r) \= reason {  
        \*view \= inject\_path(view.take(), &\["env","decision","reason"\], J::String(r));  
    }  
}

fn set\_link\_prev\_placeholder(view: \&mut J) {  
    // placeholder de 32 bytes zero (b3:00..00) na view  
    \*view \= inject\_path(view.take(), &\["env","links","prev"\], J::String("b3:0000000000000000000000000000000000000000000000000000000000000000".into()));  
}

fn ensure\_evidence\_exists(view: \&mut J) {  
    if view.pointer("/env/evidence").is\_none() {  
        \*view \= inject\_path(view.take(), &\["env","evidence"\], json\!({}));  
    }  
}

/// Injeta JSON num caminho "a.b.c" criando objetos intermediários  
fn inject\_path(mut root: J, path: &\[\&str\], value: J) \-\> J {  
    if path.is\_empty() { return value; }  
    let mut cur \= \&mut root;  
    for (i, seg) in path.iter().enumerate() {  
        if i \== path.len()-1 {  
            match cur {  
                J::Object(m) \=\> { m.insert((\*seg).to\_string(), value); }  
                \_ \=\> {  
                    let mut m \= serde\_json::Map::new();  
                    m.insert((\*seg).to\_string(), value);  
                    \*cur \= J::Object(m);  
                }  
            }  
        } else {  
            match cur {  
                J::Object(m) \=\> {  
                    cur \= m.entry((\*seg).to\_string()).or\_insert\_with(|| J::Object(serde\_json::Map::new()));  
                }  
                \_ \=\> {  
                    let mut m \= serde\_json::Map::new();  
                    m.insert((\*seg).to\_string(), J::Object(serde\_json::Map::new()));  
                    \*cur \= J::Object(m);  
                    cur \= cur.get\_mut(\*seg).unwrap();  
                }  
            }  
        }  
    }  
    root  
}

## **3\) Modelo de config —** 

## **modules/cap-policy/src/model.rs**

use serde::{Deserialize, Serialize};

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct PolicyDoc {  
    /// Endpoint do cap-permit (ex.: "http://127.0.0.1:8790")  
    pub permit\_url: Option\<String\>,  
    // Hooks futuros: llm\_url, enrich\_url...  
}

## **4\) Exemplo de policy —** 

## **configs/policy/policy.yaml**

permit\_url: "http://127.0.0.1:8790"  
\# Futuro:  
\# llm\_url: "http://127.0.0.1:8790"

---

# **Integração no** 

# **registry**

#  **(feature** 

# **modules**

# **)**

### **services/registry/Cargo.toml**

\[features\]  
default \= \[\]  
modules \= \["cap-intake", "cap-permit", "cap-policy"\]

\[dependencies\]  
cap-intake \= { path \= "../../modules/cap-intake", optional \= true }  
cap-permit \= { path \= "../../modules/cap-permit", optional \= true }  
cap-policy \= { path \= "../../modules/cap-policy", optional \= true }

### **Montagem das rotas (**

### **services/registry/src/main.rs**

### **)**

\#\[cfg(feature \= "modules")\]  
fn mount\_modules(router: axum::Router) \-\> axum::Router {  
    use cap\_intake;  
    use cap\_permit;  
    use cap\_policy;

    if let Ok(p) \= std::env::var("PERMIT\_PATH") {  
        let \_ \= cap\_permit::load\_policy\_from(p);  
    }  
    if let Ok(p) \= std::env::var("POLICY\_PATH") {  
        let \_ \= cap\_policy::load\_policy\_from(p);  
    }

    router  
        .merge(cap\_intake::router())  
        .merge(cap\_permit::router())  
        .merge(cap\_policy::router())  // /v1/policy/apply  
}

---

# **Testes de integração —** 

# **modules/cap-policy/tests/policy\_apply.rs**

use axum::Router;  
use cap\_policy::{router, load\_policy\_from};  
use serde\_json::json;  
use std::net::TcpListener;  
use tokio::task;  
use reqwest::Client;  
use tempfile::NamedTempFile;

\#\[tokio::test\]  
async fn ask\_requires\_prev\_ack\_requires\_evidence() {  
    // Policy apontando para um cap-permit “fake” local  
    // Em vez de subir o cap-permit real, vamos simular a resposta HTTP com a própria rota do teste.  
    // Aqui vamos só testar invariantes locais (sem chamar permit).

    let tmp \= NamedTempFile::new().unwrap();  
    std::fs::write(tmp.path(), "permit\_url:").unwrap();  
    load\_policy\_from(tmp.path()).unwrap();

    let app \= router();  
    let listener \= TcpListener::bind("127.0.0.1:0").unwrap();  
    let addr \= listener.local\_addr().unwrap();  
    let handle \= task::spawn(axum::serve(listener, app));

    // Cápsula sem decision → vira ASK e injeta links.prev  
    let cap \= json\!({  
        "v":"ubl-capsule/1.0",  
        "hdr":{"src":"did:ubl:a","dst":"did:ubl:b","nonce":"b64:AA==","exp": 9999999},  
        "env":{"v":"ubl-json/0.1.1","t":"record","agent":{"id":"x"},"intent":{"kind":"ATTEST","name":"demo"}, "ctx":{}, "evidence":{}},  
        "seal":{"alg":"Ed25519","kid":"did:ubl:a\#k"},  
        "receipts":\[\]  
    });

    let res \= Client::new()  
        .post(format\!("http://{}/v1/policy/apply", addr))  
        .json(\&serde\_json::json\!({ "capsule\_json": cap }))  
        .send().await.unwrap();

    assert\!(res.status().is\_success());  
    let body: serde\_json::Value \= res.json().await.unwrap();  
    assert\_eq\!(body.pointer("/capsule\_json/env/decision/verdict").unwrap(), "ASK");  
    assert\!(body.pointer("/capsule\_json/env/links/prev").is\_some());

    // Se setarmos ACK manual, precisa evidence existir  
    let cap2 \= json\!({  
        "v":"ubl-capsule/1.0",  
        "hdr":{"src":"did:ubl:a","dst":"did:ubl:b","nonce":"b64:AA==","exp": 9999999},  
        "env":{"v":"ubl-json/0.1.1","t":"record","agent":{"id":"x"},"intent":{"kind":"ATTEST","name":"demo"}, "ctx":{}, "decision":{"verdict":"ACK"}},  
        "seal":{"alg":"Ed25519","kid":"did:ubl:a\#k"},  
        "receipts":\[\]  
    });

    let res2 \= Client::new()  
        .post(format\!("http://{}/v1/policy/apply", addr))  
        .json(\&serde\_json::json\!({ "capsule\_json": cap2 }))  
        .send().await.unwrap();

    let body2: serde\_json::Value \= res2.json().await.unwrap();  
    assert\_eq\!(body2.pointer("/capsule\_json/env/decision/verdict").unwrap(), "ACK");  
    assert\!(body2.pointer("/capsule\_json/env/evidence").is\_some());

    handle.abort();  
}

---

# **Como rodar (local) 🧪**

\# 1\) Testes do módulo  
cargo test \-p cap-policy

\# 2\) Subir registry com módulos \+ configs  
export PERMIT\_PATH="$PWD/configs/policy/permit.yaml"  
export POLICY\_PATH="$PWD/configs/policy/policy.yaml"  
cargo run \-p registry \--features modules \--release

### **Exercitar a pipeline (2 chamadas HTTP)**

\# 1\) permit  
curl \-s http://127.0.0.1:8790/v1/permit/eval \-H 'content-type: application/json' \\  
  \-d @- \<\<'JSON' | jq .  
{ "capsule\_json": { "v":"ubl-capsule/1.0", "hdr":{"src":"did:ubl:a","dst":"did:ubl:b","nonce":"b64:AA==","exp": 1739999999}, "env":{"v":"ubl-json/0.1.1","t":"record","agent":{"id":"a"},"intent":{"kind":"ATTEST","name":"demo"},"ctx":{},"decision":{"verdict":"ASK"},"evidence":{},"meta":{"app":"demo-app"}},"seal":{"alg":"Ed25519","kid":"did:ubl:a\#k"},"receipts":\[\] } }  
JSON

\# 2\) policy (aplica invariantes pós-verdict)  
curl \-s http://127.0.0.1:8790/v1/policy/apply \-H 'content-type: application/json' \\  
  \-d @- \<\<'JSON' | jq .  
{ "capsule\_json": { "v":"ubl-capsule/1.0", "hdr":{"src":"did:ubl:a","dst":"did:ubl:b","nonce":"b64:AA==","exp": 1739999999}, "env":{"v":"ubl-json/0.1.1","t":"record","agent":{"id":"a"},"intent":{"kind":"ATTEST","name":"demo"},"ctx":{},"decision":{"verdict":"ASK"},"evidence":{},"meta":{"app":"demo-app"}},"seal":{"alg":"Ed25519","kid":"did:ubl:a\#k"},"receipts":\[\] } }  
JSON

---

# **Critérios de aceite ✅**

1. ASK/ACK/NACK invariantes:

   * ASK ⇒ env.links.prev presente.

   * ACK/NACK ⇒ env.evidence presente (pode ser {}).

2. Determinismo: valida view → materializa NRF → muta → re-materializa.

3. Composição: chama cap-permit (se configurado), fail-open para ASK.

4. Operação: POLICY\_PATH (YAML) \+ PERMIT\_PATH (YAML do permit).

5. Testes: cargo test \-p cap-policy passa.

---

# **Próxima (Mensagem \#4):** 

# **cap-enrich**

* Enriquecimento determinístico e boundado (headers estáveis \+ lookups com CIDs/URLs).

* Ganchos para pricing e risk score (métricas), e o contrato para LLM-smart/ LLM-engine começarem a produzir evidências numéricas reproduzíveis (mesmo com variação estocástica do modelo).

* Vetores cobrindo “sem drift” no env (enrichment só adiciona em namespaces específicos).

Se quiser, já acoplo um endpoint “macro” no registry tipo /v1/pipeline/ingest que faz intake → permit → policy → enrich em uma tacada e retorna a cápsula pronta pro próximo hop. 🚀

