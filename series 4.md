Bora\! 🚀 Mensagem \#4: cap-enrich — enriquecimento determinístico e “bounded”, preparado para plugar fontes externas (pricing/risk/etc.) sem quebrar canonicidade.

Vou te entregar: módulo completo, testes, contrato de provedores, e wire-up no registry (atrás de \--features modules), encaixando após intake → permit → policy → enrich.

---

# **Patch —** 

# **modules/cap-enrich**

## **1\)** 

## **modules/cap-enrich/Cargo.toml**

\[package\]  
name \= "cap-enrich"  
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

## **2\) Modelo/Config —** 

## **modules/cap-enrich/src/model.rs**

use serde::{Deserialize, Serialize};

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct EnrichDoc {  
    /// provedores em cadeia; execução em ordem  
    pub providers: Vec\<Provider\>,  
    /// tempo limite global por requisição (ms)  
    \#\[serde(default \= "default\_timeout\_ms")\]  
    pub timeout\_ms: u64,  
    /// tamanho máximo de payload “evidence.extra” por provedor (bytes)  
    \#\[serde(default \= "default\_max\_extra")\]  
    pub max\_extra\_bytes: usize,  
}

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct Provider {  
    pub id: String,                    // ex: "pricing.v1"  
    pub kind: ProviderKind,            // http/json | static  
    \#\[serde(default)\]  
    pub required: bool,                // falha ⇒ NACK? se false ⇒ ignora e segue  
    \#\[serde(default)\]  
    pub attach\_metrics: bool,          // copia métricas numéricas p/ env.decision.metrics  
    \#\[serde(default)\]  
    pub attach\_evidence: bool,         // adiciona CID/url/extra em env.evidence  
    \#\[serde(default)\]  
    pub namespace: Option\<String\>,     // onde gravar “enrichment.\*” (env.ctx.enrich.\*)  
    \#\[serde(default)\]  
    pub inputs: serde\_json::Value,     // parâmetros por provedor  
}

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
\#\[serde(tag \= "type")\]  
pub enum ProviderKind {  
    /// Chamada HTTP JSON: POST { capsule\_json, inputs } → {metrics?, evidence?, ctx?}  
    HttpJson { url: String },  
    /// Valor estático/mínimo: injeta em ctx/metrics direto  
    Static { value: serde\_json::Value },  
}

fn default\_timeout\_ms() \-\> u64 { 2200 }  
fn default\_max\_extra() \-\> usize { 32 \* 1024 }

## **3\) Implementação —** 

## **modules/cap-enrich/src/lib.rs**

use anyhow::{anyhow, Context, Result};  
use axum::{routing::post, Json, Router};  
use once\_cell::sync::OnceCell;  
use serde::{Deserialize, Serialize};  
use serde\_json::{json, Value as J};  
use std::{fs, path::Path};  
use time::OffsetDateTime;

mod model;  
use model::\*;

static CONF: OnceCell\<EnrichDoc\> \= OnceCell::new();

\#\[derive(Deserialize)\]  
pub struct EnrichReq {  
    /// cápsula (view JSON) pós-policy  
    capsule\_json: J,  
}

\#\[derive(Serialize)\]  
pub struct EnrichResp {  
    capsule\_json: J,  
    applied: Vec\<String\>,  
    took\_ms: u64,  
}

pub fn router() \-\> Router {  
    Router::new().route("/v1/enrich/apply", post(enrich\_handler))  
}

pub fn load\_enrich\_from\<P: AsRef\<Path\>\>(path: P) \-\> Result\<()\> {  
    let text \= fs::read\_to\_string(path)?;  
    let d: EnrichDoc \= serde\_yaml::from\_str(\&text)?;  
    CONF.set(d).ok();  
    Ok(())  
}

async fn enrich\_handler(Json(req): Json\<EnrichReq\>) \-\> Result\<Json\<EnrichResp\>, axum::http::StatusCode\> {  
    let start \= OffsetDateTime::now\_utc();

    // 1\) valida view → NRF (hard rules)  
    let mut canon \= match ubl\_json\_view::from\_json\_view(\&req.capsule\_json) {  
        Ok(v) \=\> v,  
        Err(e) \=\> {  
            // canonicidade falhou ⇒ devolve NACK  
            let mut v \= req.capsule\_json.clone();  
            force\_decision(\&mut v, "NACK", Some(format\!("canon error: {e}")));  
            return Ok(Json(EnrichResp { capsule\_json: v, applied: vec\!\[\], took\_ms: 0 }));  
        }  
    };  
    // volta a view para mutações (ctx/evidence/metrics)  
    let mut view \= ubl\_json\_view::to\_json\_view(\&canon).map\_err(|\_| axum::http::StatusCode::INTERNAL\_SERVER\_ERROR)?;

    let conf \= CONF.get().ok\_or(axum::http::StatusCode::SERVICE\_UNAVAILABLE)?;  
    let mut applied \= vec\!\[\];

    for p in \&conf.providers {  
        match apply\_provider(\&mut view, p, conf).await {  
            Ok(did) \=\> if did { applied.push(p.id.clone()); },  
            Err(e) if \!p.required \=\> {  
                // ignora erro de provedor opcional; anexa nota em evidence.meta (não canônica)  
                add\_meta\_note(\&mut view, \&p.id, \&format\!("optional provider error: {e}"));  
            }  
            Err(\_e) \=\> {  
                force\_decision(\&mut view, "NACK", Some(format\!("provider {} failed", p.id)));  
                break;  
            }  
        }  
    }

    // 2\) pós-mutate: revalidar NRF (não pode quebrar canonicalidade)  
    match ubl\_json\_view::from\_json\_view(\&view) {  
        Ok(v2) \=\> { canon \= v2; }  
        Err(e) \=\> {  
            force\_decision(\&mut view, "NACK", Some(format\!("post-canon error: {e}")));  
        }  
    }

    let took\_ms \= (OffsetDateTime::now\_utc() \- start).whole\_milliseconds() as u64;  
    Ok(Json(EnrichResp { capsule\_json: view, applied, took\_ms }))  
}

async fn apply\_provider(view: \&mut J, p: \&Provider, conf: \&EnrichDoc) \-\> Result\<bool\> {  
    match \&p.kind {  
        ProviderKind::Static { value } \=\> {  
            // injeta em ctx.enrich.\<ns\> ou ctx.enrich  
            let ns \= p.namespace.as\_deref().unwrap\_or("default");  
            inject\_ctx(view, ns, value.clone());  
            if p.attach\_metrics {  
                attach\_metrics(view, \&p.id, json\!({"static": 1}));  
            }  
            Ok(true)  
        }  
        ProviderKind::HttpJson { url } \=\> {  
            let payload \= json\!({  
              "capsule\_json": view,  
              "inputs": p.inputs  
            });  
            let cli \= reqwest::Client::builder()  
                .timeout(std::time::Duration::from\_millis(conf.timeout\_ms))  
                .build()?;  
            let res \= cli.post(url).json(\&payload).send().await?;  
            if \!res.status().is\_success() {  
                return Err(anyhow\!("http {}", res.status()));  
            }  
            let out: J \= res.json().await.context("invalid json from provider")?;  
            // contrato: { metrics?:Map\<num\>, evidence?:{cids?:\[Bytes32\], urls?:\[String\], extra?:Any}, ctx?:Any }  
            if let Some(ctxv) \= out.get("ctx").cloned() {  
                let ns \= p.namespace.as\_deref().unwrap\_or(\&p.id);  
                inject\_ctx(view, ns, ctxv);  
            }  
            if p.attach\_metrics {  
                if let Some(m) \= out.get("metrics").cloned() {  
                    attach\_metrics(view, \&p.id, m);  
                }  
            }  
            if p.attach\_evidence {  
                if let Some(ev) \= out.get("evidence") {  
                    attach\_evidence(view, ev.clone(), conf.max\_extra\_bytes)?;  
                }  
            }  
            Ok(true)  
        }  
    }  
}

fn inject\_ctx(view: \&mut J, ns: \&str, v: J) {  
    \*view \= inject\_path(view.take(), &\["env","ctx","enrich",ns\], v);  
}

fn attach\_metrics(view: \&mut J, pid: \&str, m: J) {  
    let obj \= match m {  
        J::Object(\_) \=\> m,  
        \_ \=\> json\!({"\_raw": m}),  
    };  
    \*view \= inject\_path(view.take(), &\["env","decision","metrics",pid\], obj);  
}

fn attach\_evidence(view: \&mut J, ev: J, max\_extra\_bytes: usize) \-\> Result\<()\> {  
    let mut ev\_norm \= json\!({});  
    if let Some(cids) \= ev.get("cids") { ev\_norm\["cids"\] \= cids.clone(); }  
    if let Some(urls) \= ev.get("urls") { ev\_norm\["urls"\] \= urls.clone(); }  
    if let Some(extra) \= ev.get("extra") {  
        let size \= serde\_json::to\_vec(extra)?.len();  
        if size \<= max\_extra\_bytes {  
            ev\_norm\["extra"\] \= extra.clone();  
        } else {  
            ev\_norm\["extra\_info"\] \= json\!({"truncated": true, "bytes": size});  
        }  
    }  
    // garante env.evidence existe  
    if view.pointer("/env/evidence").is\_none() {  
        \*view \= inject\_path(view.take(), &\["env","evidence"\], json\!({}));  
    }  
    // append como env.evidence.providers\[pid\]  
    \*view \= inject\_path(view.take(), &\["env","evidence","providers",pid\], ev\_norm);  
    Ok(())  
}

fn add\_meta\_note(view: \&mut J, pid: \&str, note: \&str) {  
    \*view \= inject\_path(view.take(), &\["env","meta", "notes", pid\], J::String(note.to\_string()));  
}

// util: força decision (NACK, ASK, ACK)  
fn force\_decision(view: \&mut J, verdict: \&str, reason: Option\<String\>) {  
    \*view \= inject\_path(view.take(), &\["env","decision","verdict"\], J::String(verdict.into()));  
    if let Some(r) \= reason {  
        \*view \= inject\_path(view.take(), &\["env","decision","reason"\], J::String(r));  
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

## **4\) Exemplo de config —** 

## **configs/enrich/enrich.yaml**

timeout\_ms: 2000  
max\_extra\_bytes: 8192  
providers:  
  \- id: "pricing.v1"  
    type: "HttpJson"  
    url: "http://127.0.0.1:8899/v1/pricing/eval"  
    required: false  
    attach\_metrics: true  
    attach\_evidence: true  
    namespace: "pricing"  
    inputs:  
      currency: "USD"  
      country: "US"

  \- id: "risk.static"  
    type: "Static"  
    required: false  
    attach\_metrics: true  
    namespace: "risk"  
    value:  
      risk\_band: "LOW"  
      risk\_score: 0.12

---

# **Integração no** 

# **registry**

#  **(feature** 

# **modules**

# **)**

### **services/registry/Cargo.toml**

\[features\]  
default \= \[\]  
modules \= \["cap-intake", "cap-permit", "cap-policy", "cap-enrich"\]

\[dependencies\]  
cap-intake \= { path \= "../../modules/cap-intake", optional \= true }  
cap-permit \= { path \= "../../modules/cap-permit", optional \= true }  
cap-policy \= { path \= "../../modules/cap-policy", optional \= true }  
cap-enrich \= { path \= "../../modules/cap-enrich", optional \= true }

### **Montagem de rotas (trecho) —** 

### **services/registry/src/main.rs**

\#\[cfg(feature \= "modules")\]  
fn mount\_modules(router: axum::Router) \-\> axum::Router {  
    use cap\_intake;  
    use cap\_permit;  
    use cap\_policy;  
    use cap\_enrich;

    if let Ok(p) \= std::env::var("PERMIT\_PATH") { let \_ \= cap\_permit::load\_policy\_from(p); }  
    if let Ok(p) \= std::env::var("POLICY\_PATH") { let \_ \= cap\_policy::load\_policy\_from(p); }  
    if let Ok(p) \= std::env::var("ENRICH\_PATH") { let \_ \= cap\_enrich::load\_enrich\_from(p); }

    router  
        .merge(cap\_intake::router())  
        .merge(cap\_permit::router())  
        .merge(cap\_policy::router())  
        .merge(cap\_enrich::router()) // /v1/enrich/apply  
}

---

# **Teste de integração —** 

# **modules/cap-enrich/tests/enrich\_apply.rs**

use axum::Router;  
use cap\_enrich::{router, load\_enrich\_from};  
use serde\_json::json;  
use std::net::TcpListener;  
use tokio::task;  
use tempfile::NamedTempFile;  
use reqwest::Client;

\#\[tokio::test\]  
async fn static\_and\_optional\_http\_enrich() {  
    // config com 1 estático \+ 1 http (url inválida, optional ⇒ não falha)  
    let text \= r\#"  
timeout\_ms: 250  
providers:  
  \- id: "risk.static"  
    type: "Static"  
    required: false  
    attach\_metrics: true  
    namespace: "risk"  
    value: { risk\_score: 0.1 }  
  \- id: "pricing.v1"  
    type: "HttpJson"  
    url: "http://127.0.0.1:65530/v1/not-running"  
    required: false  
    attach\_metrics: true  
    attach\_evidence: true  
    namespace: "pricing"  
"\#;  
    let tmp \= NamedTempFile::new().unwrap();  
    std::fs::write(tmp.path(), text).unwrap();  
    load\_enrich\_from(tmp.path()).unwrap();

    let app \= router();  
    let listener \= TcpListener::bind("127.0.0.1:0").unwrap();  
    let addr \= listener.local\_addr().unwrap();  
    let handle \= task::spawn(axum::serve(listener, app));

    let cap \= json\!({  
      "v":"ubl-capsule/1.0",  
      "hdr":{"src":"did:ubl:a","dst":"did:ubl:b","nonce":"b64:AA==","exp": 1739999999},  
      "env":{"v":"ubl-json/0.1.1","t":"record","agent":{"id":"a"},"intent":{"kind":"ATTEST","name":"demo"},"ctx":{},"decision":{"verdict":"ASK"},"evidence":{}},  
      "seal":{"alg":"Ed25519","kid":"did:ubl:a\#k"},  
      "receipts":\[\]  
    });

    let res \= Client::new()  
        .post(format\!("http://{}/v1/enrich/apply", addr))  
        .json(\&serde\_json::json\!({ "capsule\_json": cap }))  
        .send().await.unwrap();

    assert\!(res.status().is\_success());  
    let body: serde\_json::Value \= res.json().await.unwrap();

    // enrichment estático aplicado  
    assert\_eq\!(body.pointer("/capsule\_json/env/ctx/enrich/risk/risk\_score").unwrap(), 0.1);

    // provedor http opcional deu erro ⇒ adiciona meta note, mas não falha  
    assert\!(body.pointer("/capsule\_json/env/meta/notes/pricing.v1").is\_some());

    handle.abort();  
}

---

# **Como usar localmente 🧪**

\# 1\) Rodar testes do módulo  
cargo test \-p cap-enrich

\# 2\) Subir registry com módulos \+ políticas  
export PERMIT\_PATH="$PWD/configs/policy/permit.yaml"  
export POLICY\_PATH="$PWD/configs/policy/policy.yaml"  
export ENRICH\_PATH="$PWD/configs/enrich/enrich.yaml"  
cargo run \-p registry \--features modules \--release

### **Exercício “pipeline” manual (curl)**

\# após intake/permit/policy, chama enrich  
curl \-s http://127.0.0.1:8790/v1/enrich/apply \-H 'content-type: application/json' \\  
  \-d @- \<\<'JSON' | jq .  
{ "capsule\_json": { "v":"ubl-capsule/1.0", "hdr":{"src":"did:ubl:a","dst":"did:ubl:b","nonce":"b64:AA==","exp": 1739999999}, "env":{"v":"ubl-json/0.1.1","t":"record","agent":{"id":"a"},"intent":{"kind":"ATTEST","name":"demo"},"ctx":{},"decision":{"verdict":"ASK"},"evidence":{}}, "seal":{"alg":"Ed25519","kid":"did:ubl:a\#k"},"receipts":\[\] } }  
JSON

---

## **Contrato dos provedores (para LLM-engine/LLM-smart e “pricing”)**

* Entrada (POST): {"capsule\_json": \<view\>, "inputs": \<obj\>}

* Saída:

{  
  "metrics": { "price": 123.45, "p95\_ms": 88 },      // números → vão para env.decision.metrics.\<provider\>  
  "evidence": { "cids": \["b3:..."\], "urls": \["..."\], "extra": {"sample": "bounded"} }, // anexado em env.evidence.providers.\<provider\>  
  "ctx": { "offer": { "id":"A", "price": 123.45 } }  // incorporado em env.ctx.enrich.\<namespace|provider\>  
}

* 

* Bounded:

  * extra é truncado por max\_extra\_bytes (default 32 KiB).

  * Timeout por chamada \= timeout\_ms (default 2200 ms).

  * Se required=false, erro do provedor não derruba a cápsula (anota em env.meta.notes.\<provider\>).

---

## **Critérios de aceite ✅**

1. Determinismo canônico: enrich muta apenas env.ctx.enrich.\*, env.decision.metrics.\*, env.evidence.providers.\*, env.meta.notes.\*. Re-materialização NRF sempre válida.

2. Limites claros: timeout, truncagem de extra, erro opcional não derruba pipeline.

3. Composição: encadeável após cap-policy; preparado para pricing real, LLM-engine/LLM-smart e afins.

4. Testes: cargo test \-p cap-enrich passa (inclui cenário com provedor HTTP indisponível opcional).

---

## **Próxima (Mensagem \#5):** 

## **cap-transport**

* Derivar headers de transporte (fora do canônico) com base em hdr.\* e policy (retry/route/ttl derivado de exp), e simular 2 hops com receipts encadeadas integradas (assinatura hop domain “ubl-receipt/1.0”).

* Entregar vetores de integração intake → permit → policy → enrich → transport → add\_receipt.

Se quiser, também já monto um endpoint macro do registry /v1/pipeline/ingest que faz a orquestração end-to-end e retorna a cápsula pronta pro próximo hop. Quer que eu já inclua esse endpoint agora? 😄🧩

