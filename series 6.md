Bora, Dan — Mensagem \#6: cap-llm 🤖⚡

Módulo LLM com 2 backends (premium e OSS), pronto pra inference, eval e benchmark real (usa API keys via env, sem gravar segredo). Inclui integração no registry, configs, e testes (mock \+ live gated).

---

# **Patch —** 

# **modules/cap-llm**

## **1\)** 

## **modules/cap-llm/Cargo.toml**

\[package\]  
name \= "cap-llm"  
version \= "0.1.0"  
edition \= "2021"  
license \= "MIT OR Apache-2.0"

\[dependencies\]  
anyhow \= "1"  
axum \= { version \= "0.7", features \= \["json"\] }  
serde \= { version \= "1", features \= \["derive"\] }  
serde\_json \= "1"  
serde\_yaml \= "0.9"  
reqwest \= { version \= "0.12", features \= \["json", "gzip", "brotli", "zstd", "stream"\] }  
tokio \= { version \= "1", features \= \["macros", "rt-multi-thread", "time"\] }  
tracing \= "0.1"  
once\_cell \= "1"  
time \= { version \= "0.3", features \= \["macros"\] }  
uuid \= { version \= "1", features \= \["v4"\] }  
url \= "2"  
regex \= "1"

\# Base/view (para opcionalmente contextualizar com cápsulas)  
ubl\_json\_view \= { path \= "../../impl/rust/ubl\_json\_view" }

\[dev-dependencies\]  
httptest \= "0.15"      \# mock http server  
tempfile \= "3"

---

## **2\) Config —** 

## **modules/cap-llm/src/config.rs**

use serde::{Deserialize, Serialize};

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct LlmConfig {  
    /// Tabela de provedores nomeados (ex.: "premium", "oss", "lab").  
    pub providers: std::collections::BTreeMap\<String, Provider\>,  
    /// Preços por mil tokens (overrideável por provider/model).  
    \#\[serde(default)\]  
    pub pricing: PricingTable,  
    /// Limites de benchmark (concorrência, timeout, repeats)  
    \#\[serde(default)\]  
    pub bench: BenchConfig,  
}

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct Provider {  
    /// "openai-compatible" | "ollama"  
    pub kind: String,  
    /// URL base (ex.: https://api.openai.com/v1 | http://127.0.0.1:11434)  
    pub endpoint: String,  
    /// Nome da env var que contém a API key (apenas para openai-compatible).  
    \#\[serde(default)\]  
    pub api\_key\_env: Option\<String\>,  
    /// Header adicional (opcional), ex.: {"X-API-KEY":"env:FOO\_KEY"}  
    \#\[serde(default)\]  
    pub extra\_headers: std::collections::BTreeMap\<String, String\>,  
    /// Timeout em ms para requests.  
    \#\[serde(default \= "default\_timeout\_ms")\]  
    pub timeout\_ms: u64,  
    /// Model default deste provider (pode ser override no request).  
    \#\[serde(default)\]  
    pub default\_model: Option\<String\>,  
    /// Pricing override específico por modelo (por 1k tokens).  
    \#\[serde(default)\]  
    pub pricing: PricingTable,  
}

\#\[derive(Debug, Clone, Default, Serialize, Deserialize)\]  
pub struct PricingTable {  
    /// input e output, em USD por 1000 tokens  
    \#\[serde(default)\]  
    pub input\_per\_1k: Option\<f64\>,  
    \#\[serde(default)\]  
    pub output\_per\_1k: Option\<f64\>,  
    /// overrides por modelo (name \-\> {input\_per\_1k, output\_per\_1k})  
    \#\[serde(default)\]  
    pub by\_model: std::collections::BTreeMap\<String, PricingTable\>,  
}

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct BenchConfig {  
    \#\[serde(default \= "default\_concurrency")\] pub concurrency: usize,  
    \#\[serde(default \= "default\_repeats")\] pub repeats: usize,  
    \#\[serde(default \= "default\_timeout\_ms")\] pub timeout\_ms: u64,  
}  
fn default\_timeout\_ms() \-\> u64 { 60000 }  
fn default\_concurrency() \-\> usize { 4 }  
fn default\_repeats() \-\> usize { 3 }

---

## **3\) API —** 

## **modules/cap-llm/src/api.rs**

use serde::{Deserialize, Serialize};  
use serde\_json::Value as J;

\#\[derive(Debug, Clone, Deserialize)\]  
pub struct CompleteReq {  
    /// "premium" | "oss" | "lab" (chave do provider no config)  
    pub provider: String,  
    /// opcional: override do modelo; se ausente, usa default do provider  
    pub model: Option\<String\>,  
    /// prompt (texto plano); se preferir mensagens, usar messages\[\]  
    pub prompt: Option\<String\>,  
    /// chat messages (openai-style)  
    pub messages: Option\<Vec\<ChatMsg\>\>,  
    /// parâmetros de sampling  
    \#\[serde(default)\] pub params: SampleParams,  
    /// opcional: cápsula para contexto (env/hdr), sem tocar canônico  
    \#\[serde(default)\] pub capsule\_json: Option\<J\>,  
}

\#\[derive(Debug, Clone, Serialize)\]  
pub struct CompleteResp {  
    pub provider: String,  
    pub model: String,  
    pub completion: String,  
    pub usage: Usage,  
    pub latency\_ms: u128,  
    pub price\_usd\_est: Option\<f64\>,  
    pub raw: J,  
}

\#\[derive(Debug, Clone, Deserialize, Serialize)\]  
pub struct ChatMsg { pub role: String, pub content: String }

\#\[derive(Debug, Clone, Deserialize, Serialize, Default)\]  
pub struct SampleParams {  
    \#\[serde(default)\] pub temperature: Option\<f32\>,  
    \#\[serde(default)\] pub top\_p: Option\<f32\>,  
    \#\[serde(default)\] pub max\_tokens: Option\<u32\>,  
    \#\[serde(default)\] pub stream: Option\<bool\>,  
}

\#\[derive(Debug, Clone, Deserialize, Serialize, Default)\]  
pub struct Usage {  
    pub prompt\_tokens: Option\<u32\>,  
    pub completion\_tokens: Option\<u32\>,  
    pub total\_tokens: Option\<u32\>,  
}

\#\[derive(Debug, Clone, Deserialize)\]  
pub struct BenchReq {  
    pub cases: Vec\<BenchCase\>,  
    pub repetitions: Option\<usize\>,  
    pub concurrency: Option\<usize\>,  
    /// se true, falha se qualquer chamada der erro  
    \#\[serde(default)\] pub fail\_on\_error: bool,  
}

\#\[derive(Debug, Clone, Deserialize)\]  
pub struct BenchCase {  
    pub name: String,  
    pub provider: String,  
    pub model: Option\<String\>,  
    pub prompt: Option\<String\>,  
    pub messages: Option\<Vec\<ChatMsg\>\>,  
    \#\[serde(default)\] pub params: SampleParams,  
}

\#\[derive(Debug, Clone, Serialize)\]  
pub struct BenchResult {  
    pub summary: serde\_json::Value,  
    pub runs: Vec\<CompleteResp\>,  
}

---

## **4\) Implementação —** 

## **modules/cap-llm/src/lib.rs**

mod config;  
mod api;  
mod providers;

use anyhow::{anyhow, Result};  
use axum::{routing::post, Json, Router};  
use once\_cell::sync::OnceCell;  
use serde\_json::{json, Value as J};  
use std::time::Instant;

pub use api::\*;  
pub use config::\*;

static CONF: OnceCell\<LlmConfig\> \= OnceCell::new();

pub fn load\_llm\_from(path: impl AsRef\<std::path::Path\>) \-\> Result\<()\> {  
    let text \= std::fs::read\_to\_string(path)?;  
    let doc: LlmConfig \= serde\_yaml::from\_str(\&text)?;  
    CONF.set(doc).ok();  
    Ok(())  
}

pub fn router() \-\> Router {  
    Router::new()  
        .route("/v1/llm/complete", post(complete\_handler))  
        .route("/v1/llm/bench", post(bench\_handler))  
        // util simples p/ “eval” textual (judge local via regex/keywords)  
        .route("/v1/llm/judge", post(judge\_handler))  
}

// \=== /complete \===  
async fn complete\_handler(Json(req): Json\<CompleteReq\>) \-\> Result\<Json\<CompleteResp\>, axum::http::StatusCode\> {  
    let conf \= CONF.get().ok\_or(axum::http::StatusCode::SERVICE\_UNAVAILABLE)?;  
    let prov \= conf.providers.get(\&req.provider).ok\_or(axum::http::StatusCode::BAD\_REQUEST)?;

    let model \= req.model.clone().or\_else(|| prov.default\_model.clone())  
        .ok\_or(axum::http::StatusCode::BAD\_REQUEST)?;

    let started \= Instant::now();  
    let out \= providers::run\_completion(prov, \&model, \&req).await  
        .map\_err(|\_| axum::http::StatusCode::BAD\_GATEWAY)?;  
    let latency\_ms \= started.elapsed().as\_millis();

    let usage \= out.usage.clone().unwrap\_or\_default();  
    let price \= estimate\_price\_usd(\&conf, prov, \&model, \&usage);

    Ok(Json(CompleteResp {  
        provider: req.provider,  
        model,  
        completion: out.text,  
        usage,  
        latency\_ms,  
        price\_usd\_est: price,  
        raw: out.raw,  
    }))  
}

// \=== /bench \===  
async fn bench\_handler(Json(req): Json\<BenchReq\>) \-\> Result\<Json\<BenchResult\>, axum::http::StatusCode\> {  
    use futures::stream::{FuturesUnordered, StreamExt};

    let conf \= CONF.get().ok\_or(axum::http::StatusCode::SERVICE\_UNAVAILABLE)?;  
    let repeats \= req.repetitions.unwrap\_or(conf.bench.repeats);  
    let conc \= req.concurrency.unwrap\_or(conf.bench.concurrency);

    let mut runs \= Vec::new();  
    let mut tasks \= FuturesUnordered::new();  
    let sem \= std::sync::Arc::new(tokio::sync::Semaphore::new(conc as \_));

    for case in \&req.cases {  
        for \_ in 0..repeats {  
            let permit \= sem.clone().acquire\_owned().await.unwrap();  
            let case \= case.clone();  
            let conf \= CONF.get().unwrap().clone();  
            tasks.push(tokio::spawn(async move {  
                let \_p \= permit;  
                let prov \= conf.providers.get(\&case.provider).ok\_or\_else(|| anyhow\!("unknown provider"))?;  
                let model \= case.model.clone().or\_else(|| prov.default\_model.clone()).ok\_or\_else(|| anyhow\!("no model"))?;  
                let started \= Instant::now();  
                let req \= crate::api::CompleteReq {  
                    provider: case.provider.clone(),  
                    model: Some(model.clone()),  
                    prompt: case.prompt.clone(),  
                    messages: case.messages.clone(),  
                    params: case.params.clone(),  
                    capsule\_json: None,  
                };  
                let out \= providers::run\_completion(prov, \&model, \&req).await?;  
                let latency\_ms \= started.elapsed().as\_millis();  
                let usage \= out.usage.clone().unwrap\_or\_default();  
                let price \= estimate\_price\_usd(\&conf, prov, \&model, \&usage);  
                let resp \= crate::api::CompleteResp {  
                    provider: case.provider.clone(),  
                    model,  
                    completion: out.text,  
                    usage,  
                    latency\_ms,  
                    price\_usd\_est: price,  
                    raw: out.raw,  
                };  
                Ok::\<\_, anyhow::Error\>(resp)  
            }));  
        }  
    }

    while let Some(res) \= tasks.next().await {  
        match res {  
            Ok(Ok(r)) \=\> runs.push(r),  
            Ok(Err(e)) \=\> {  
                if req.fail\_on\_error { return Err(axum::http::StatusCode::BAD\_GATEWAY); }  
                tracing::warn\!("bench run error: {e:?}");  
            }  
            Err(e) \=\> {  
                if req.fail\_on\_error { return Err(axum::http::StatusCode::BAD\_GATEWAY); }  
                tracing::warn\!("join error: {e:?}");  
            }  
        }  
    }

    // sumariza por (provider, model)  
    let mut buckets: std::collections::BTreeMap\<(String,String), Vec\<\&CompleteResp\>\> \= std::collections::BTreeMap::new();  
    for r in \&runs {  
        buckets.entry((r.provider.clone(), r.model.clone())).or\_default().push(r);  
    }  
    let mut summary \= vec\!\[\];  
    for ((p,m), items) in buckets {  
        let n \= items.len() as f64;  
        let lat: Vec\<f64\> \= items.iter().map(|r| r.latency\_ms as f64).collect();  
        let lat\_avg \= lat.iter().sum::\<f64\>() / n;  
        let price: Vec\<f64\> \= items.iter().filter\_map(|r| r.price\_usd\_est).collect();  
        let price\_avg \= if price.is\_empty() { None } else { Some(price.iter().sum::\<f64\>() / price.len() as f64) };  
        summary.push(json\!({  
            "provider": p, "model": m,  
            "count": items.len(),  
            "latency\_ms\_avg": lat\_avg,  
            "price\_usd\_avg": price\_avg,  
        }));  
    }

    Ok(Json(BenchResult {  
        summary: J::Array(summary),  
        runs,  
    }))  
}

// \=== /judge (heurística local, barata)  
use axum::Json as AxJson;  
use serde::Deserialize;  
\#\[derive(Deserialize)\]  
struct JudgeReq { expected\_keywords: Vec\<String\>, text: String }  
\#\[derive(serde::Serialize)\]  
struct JudgeResp { ok: bool, missing: Vec\<String\> }

async fn judge\_handler(AxJson(req): AxJson\<JudgeReq\>) \-\> Result\<AxJson\<JudgeResp\>, axum::http::StatusCode\> {  
    let mut missing \= vec\!\[\];  
    for k in \&req.expected\_keywords {  
        if \!req.text.to\_lowercase().contains(\&k.to\_lowercase()) {  
            missing.push(k.clone());  
        }  
    }  
    Ok(AxJson(JudgeResp { ok: missing.is\_empty(), missing }))  
}

// \=== pricing  
fn estimate\_price\_usd(conf: \&LlmConfig, prov: \&Provider, model: \&str, usage: \&crate::api::Usage) \-\> Option\<f64\> {  
    // prioridade: provider.pricing.by\_model \-\> conf.pricing.by\_model \-\> provider.pricing \-\> conf.pricing  
    let source \= prov.pricing.by\_model.get(model)  
        .or\_else(|| conf.pricing.by\_model.get(model))  
        .cloned()  
        .unwrap\_or\_else(|| merge\_pricing(\&prov.pricing, \&conf.pricing));

    let inp \= usage.prompt\_tokens? as f64 / 1000.0 \* source.input\_per\_1k?;  
    let out \= usage.completion\_tokens? as f64 / 1000.0 \* source.output\_per\_1k?;  
    Some(inp \+ out)  
}

fn merge\_pricing(a: \&PricingTable, b: \&PricingTable) \-\> PricingTable {  
    PricingTable {  
        input\_per\_1k: a.input\_per\_1k.or(b.input\_per\_1k),  
        output\_per\_1k: a.output\_per\_1k.or(b.output\_per\_1k),  
        by\_model: a.by\_model.clone(), // simples  
    }  
}

---

## **5\) Providers —** 

## **modules/cap-llm/src/providers.rs**

use anyhow::{anyhow, Result};  
use serde\_json::{json, Value as J};  
use crate::{Provider, CompleteReq};  
use tokio::time::{timeout, Duration};  
use regex::Regex;

\#\[derive(Clone, Debug)\]  
pub struct CompletionOut { pub text: String, pub usage: Option\<crate::api::Usage\>, pub raw: J }

pub async fn run\_completion(prov: \&Provider, model: \&str, req: \&CompleteReq) \-\> Result\<CompletionOut\> {  
    match prov.kind.as\_str() {  
        "openai-compatible" \=\> run\_openai\_compatible(prov, model, req).await,  
        "ollama" \=\> run\_ollama(prov, model, req).await,  
        other \=\> Err(anyhow\!("unsupported provider kind: {other}")),  
    }  
}

fn bearer\_from\_env(name: Option\<\&String\>) \-\> Option\<String\> {  
    name.and\_then(|k| std::env::var(k).ok()).map(|v| format\!("Bearer {v}"))  
}

pub async fn run\_openai\_compatible(prov: \&Provider, model: \&str, req: \&CompleteReq) \-\> Result\<CompletionOut\> {  
    let to \= Duration::from\_millis(prov.timeout\_ms);  
    let client \= reqwest::Client::builder()  
        .gzip(true).brotli(true).zstd(true)  
        .build()?;

    // headers  
    let mut headers \= reqwest::header::HeaderMap::new();  
    if let Some(b) \= bearer\_from\_env(prov.api\_key\_env.as\_ref()) {  
        headers.insert(reqwest::header::AUTHORIZATION, b.parse().unwrap());  
    }  
    for (k,v) in \&prov.extra\_headers {  
        if let Some(v) \= v.strip\_prefix("env:") {  
            if let Ok(val) \= std::env::var(v) {  
                headers.insert(k.as\_str(), val.parse().unwrap());  
                continue;  
            }  
        }  
        headers.insert(k.as\_str(), v.parse().unwrap());  
    }

    // body  
    let mut body \= json\!({  
      "model": model,  
      "temperature": req.params.temperature,  
      "top\_p": req.params.top\_p,  
      "max\_tokens": req.params.max\_tokens,  
      "stream": false  
    });  
    if let Some(p) \= \&req.prompt {  
        body.as\_object\_mut().unwrap().insert("prompt".into(), J::String(p.clone()));  
    }  
    if let Some(msgs) \= \&req.messages {  
        body.as\_object\_mut().unwrap().insert("messages".into(), serde\_json::to\_value(msgs)?);  
    }

    let url \= format\!("{}/chat/completions", prov.endpoint.trim\_end\_matches('/')); // funciona p/ openai-like  
    let res \= timeout(to, client.post(url).headers(headers).json(\&body).send()).await??;  
    if \!res.status().is\_success() {  
        return Err(anyhow\!("upstream status {}", res.status()));  
    }  
    let raw: J \= res.json().await?;  
    // tenta extrair texto/usage em formatos comuns  
    let text \= raw.pointer("/choices/0/message/content")  
        .and\_then(|v| v.as\_str()).map(|s| s.to\_string())  
        .or\_else(|| raw.pointer("/choices/0/text").and\_then(|v| v.as\_str().map(|s| s.to\_string())))  
        .unwrap\_or\_default();

    let usage \= crate::api::Usage {  
        prompt\_tokens: raw.pointer("/usage/prompt\_tokens").and\_then(|v| v.as\_u64()).map(|x| x as u32),  
        completion\_tokens: raw.pointer("/usage/completion\_tokens").and\_then(|v| v.as\_u64()).map(|x| x as u32),  
        total\_tokens: raw.pointer("/usage/total\_tokens").and\_then(|v| v.as\_u64()).map(|x| x as u32),  
    };  
    Ok(CompletionOut { text, usage: Some(usage), raw })  
}

pub async fn run\_ollama(prov: \&Provider, model: \&str, req: \&CompleteReq) \-\> Result\<CompletionOut\> {  
    let to \= Duration::from\_millis(prov.timeout\_ms);  
    let client \= reqwest::Client::new();  
    let url \= format\!("{}/api/generate", prov.endpoint.trim\_end\_matches('/'));

    // Ollama "generate" é prompt-based; para chat, usa /api/chat  
    let prompt \= req.prompt.clone().unwrap\_or\_else(|| {  
        let msg \= req.messages.as\_ref().and\_then(|m| m.last());  
        msg.map(|m| m.content.clone()).unwrap\_or\_default()  
    });

    let body \= json\!({  
        "model": model,  
        "prompt": prompt,  
        "stream": false,  
        "options": {  
            "temperature": req.params.temperature,  
            "top\_p": req.params.top\_p,  
            "num\_predict": req.params.max\_tokens  
        }  
    });

    let res \= timeout(to, client.post(url).json(\&body).send()).await??;  
    if \!res.status().is\_success() {  
        return Err(anyhow\!("ollama status {}", res.status()));  
    }  
    let raw: J \= res.json().await?;  
    let text \= raw.get("response").and\_then(|v| v.as\_str()).unwrap\_or\_default().to\_string();

    // Ollama não traz usage canonical; podemos heurística (tokens≈palavras\*1.3)  
    let est\_toks \= estimate\_tokens(\&text);  
    let usage \= crate::api::Usage {  
        prompt\_tokens: None, completion\_tokens: Some(est\_toks), total\_tokens: None  
    };  
    Ok(CompletionOut { text, usage: Some(usage), raw })  
}

fn estimate\_tokens(s: \&str) \-\> u32 {  
    let re \= Regex::new(r"\\w+").unwrap();  
    let words \= re.find\_iter(s).count() as f64;  
    (words \* 1.3).round() as u32  
}

---

## **6\) Config YAML —** 

## **configs/llm/llm.yaml**

providers:  
  premium:  
    kind: openai-compatible  
    endpoint: https://api.openai.com/v1  
    api\_key\_env: OPENAI\_API\_KEY  
    timeout\_ms: 70000  
    default\_model: gpt-4o-mini  
    pricing:  
      by\_model:  
        gpt-4o-mini:  
          input\_per\_1k: 0.15  
          output\_per\_1k: 0.60

  oss:  
    kind: ollama  
    endpoint: http://127.0.0.1:11434  
    timeout\_ms: 90000  
    default\_model: llama3.1:8b

pricing:  
  input\_per\_1k: 0.00  
  output\_per\_1k: 0.00

bench:  
  concurrency: 4  
  repeats: 3  
  timeout\_ms: 70000

---

## **7\) Integração no** 

## **registry**

### **services/registry/Cargo.toml**

\[features\]  
default \= \[\]  
modules \= \[  
  "cap-intake",  
  "cap-permit",  
  "cap-policy",  
  "cap-enrich",  
  "cap-transport",  
  "cap-llm",  
\]

\[dependencies\]  
cap-intake    \= { path \= "../../modules/cap-intake",    optional \= true }  
cap-permit    \= { path \= "../../modules/cap-permit",    optional \= true }  
cap-policy    \= { path \= "../../modules/cap-policy",    optional \= true }  
cap-enrich    \= { path \= "../../modules/cap-enrich",    optional \= true }  
cap-transport \= { path \= "../../modules/cap-transport", optional \= true }  
cap-llm       \= { path \= "../../modules/cap-llm",       optional \= true }

### **services/registry/src/main.rs**

###  **(trecho)**

\#\[cfg(feature \= "modules")\]  
fn mount\_modules(router: axum::Router) \-\> axum::Router {  
    use cap\_intake;  
    use cap\_permit;  
    use cap\_policy;  
    use cap\_enrich;  
    use cap\_transport;  
    use cap\_llm;

    if let Ok(p) \= std::env::var("PERMIT\_PATH")    { let \_ \= cap\_permit::load\_policy\_from(p); }  
    if let Ok(p) \= std::env::var("POLICY\_PATH")    { let \_ \= cap\_policy::load\_policy\_from(p); }  
    if let Ok(p) \= std::env::var("ENRICH\_PATH")    { let \_ \= cap\_enrich::load\_enrich\_from(p); }  
    if let Ok(p) \= std::env::var("TRANSPORT\_PATH") { let \_ \= cap\_transport::load\_transport\_from(p); }  
    if let Ok(p) \= std::env::var("LLM\_PATH")       { let \_ \= cap\_llm::load\_llm\_from(p); }

    router  
        .merge(cap\_intake::router())  
        .merge(cap\_permit::router())  
        .merge(cap\_policy::router())  
        .merge(cap\_enrich::router())  
        .merge(cap\_transport::router())  
        .merge(cap\_llm::router())  // /v1/llm/complete | /v1/llm/bench | /v1/llm/judge  
}

---

## **8\) Testes**

### **Mock test (não precisa internet) —** 

### **modules/cap-llm/tests/mock\_complete.rs**

use axum::{routing::post, Json, Router};  
use cap\_llm::{load\_llm\_from, router as llm\_router};  
use httptest::{matchers::\*, responders::\*, Expectation, Server};  
use serde\_json::json;  
use tempfile::NamedTempFile;  
use tokio::task;

\#\[tokio::test\]  
async fn openai\_compatible\_mock\_works() {  
    // mock upstream  
    let srv \= Server::run();  
    srv.expect(  
        Expectation::matching(all\_of\!\[  
            request::method\_path("POST", "/v1/chat/completions"),  
            request::headers(contains("authorization"))  
        \])  
        .respond\_with(json\_encoded(json\!({  
            "choices":\[{"message":{"content":"Hello Dan\!"}}\],  
            "usage":{"prompt\_tokens":5,"completion\_tokens":3,"total\_tokens":8}  
        })))  
    );

    // config  
    let cfg \= format\!(r\#"  
providers:  
  premium:  
    kind: openai-compatible  
    endpoint: "{}"  
    api\_key\_env: TEST\_KEY  
    timeout\_ms: 10000  
    default\_model: gpt-4o-mini  
"\#, srv.url\_str("/v1"));  
    std::env::set\_var("TEST\_KEY", "sekret");  
    let tmp \= NamedTempFile::new().unwrap();  
    std::fs::write(tmp.path(), cfg).unwrap();  
    load\_llm\_from(tmp.path()).unwrap();

    let app \= llm\_router();  
    let listener \= std::net::TcpListener::bind("127.0.0.1:0").unwrap();  
    let addr \= listener.local\_addr().unwrap();  
    let handle \= task::spawn(axum::serve(listener, app));

    // call complete  
    let req \= json\!({  
      "provider":"premium",  
      "model":"gpt-4o-mini",  
      "prompt":"hi",  
      "params":{"max\_tokens":16}  
    });  
    let out: serde\_json::Value \=  
        reqwest::Client::new().post(format\!("http://{}/v1/llm/complete", addr))  
        .json(\&req).send().await.unwrap().json().await.unwrap();

    assert\_eq\!(out\["completion"\], "Hello Dan\!");  
    assert\_eq\!(out\["usage"\]\["completion\_tokens"\], 3);

    handle.abort();  
}

### **Live bench (opt-in) —** 

### **modules/cap-llm/tests/live\_bench.rs**

Roda apenas se LIVE\_LLM=1 e a(s) env var(s) de API estiver(em) setadas.  
use cap\_llm::{load\_llm\_from, router as llm\_router};  
use serde\_json::json;  
use tempfile::NamedTempFile;  
use tokio::task;

\#\[tokio::test\]  
async fn live\_bench\_if\_enabled() {  
    if std::env::var("LIVE\_LLM").ok().as\_deref() \!= Some("1") { return; }  
    // exige OPENAI\_API\_KEY se for usar premium; ajuste conforme seu cfg  
    if std::env::var("OPENAI\_API\_KEY").is\_err() { return; }

    let cfg \= r\#"  
providers:  
  premium:  
    kind: openai-compatible  
    endpoint: https://api.openai.com/v1  
    api\_key\_env: OPENAI\_API\_KEY  
    default\_model: gpt-4o-mini  
bench: { concurrency: 2, repeats: 2 }  
"\#;  
    let tmp \= NamedTempFile::new().unwrap();  
    std::fs::write(tmp.path(), cfg).unwrap();  
    load\_llm\_from(tmp.path()).unwrap();

    let app \= llm\_router();  
    let listener \= std::net::TcpListener::bind("127.0.0.1:0").unwrap();  
    let addr \= listener.local\_addr().unwrap();  
    let handle \= task::spawn(axum::serve(listener, app));

    let bench \= json\!({  
      "cases":\[  
        {"name":"hello","provider":"premium","prompt":"Say 'hi Dan' in one short sentence"}  
      \],  
      "repetitions": 2,  
      "concurrency": 2,  
      "fail\_on\_error": true  
    });  
    let out: serde\_json::Value \=  
      reqwest::Client::new().post(format\!("http://{}/v1/llm/bench", addr))  
        .json(\&bench).send().await.unwrap().json().await.unwrap();

    assert\!(out\["runs"\].as\_array().unwrap().len() \>= 1);  
    handle.abort();  
}

---

## **9\) Uso rápido (curl)**

Complete (OpenAI-compatible):

export LLM\_PATH="$PWD/configs/llm/llm.yaml"  
export OPENAI\_API\_KEY=sk-your-key  
cargo run \-p registry \--features modules \--release  
\# nova aba:  
curl \-s localhost:8790/v1/llm/complete \-H 'content-type: application/json' \-d '{  
  "provider":"premium",  
  "model":"gpt-4o-mini",  
  "prompt":"Resuma em 1 frase o valor da UBL Capsule v1"  
}' | jq .

Complete (Ollama local):

curl \-s localhost:8790/v1/llm/complete \-H 'content-type: application/json' \-d '{  
  "provider":"oss",  
  "model":"llama3.1:8b",  
  "prompt":"Explique em 1 frase o que é ai-nrf1"  
}' | jq .

Benchmark real (não determinístico):

curl \-s localhost:8790/v1/llm/bench \-H 'content-type: application/json' \-d '{  
  "cases":\[  
    {"name":"pricing-title","provider":"premium","model":"gpt-4o-mini","prompt":"Give a catchy 5-word title for a pricing page."},  
    {"name":"pricing-title","provider":"oss","model":"llama3.1:8b","prompt":"Give a catchy 5-word title for a pricing page."}  
  \],  
  "repetitions": 3,  
  "concurrency": 3,  
  "fail\_on\_error": false  
}' | jq .

---

## **10\) Critérios de aceite ✅**

* Dois backends funcionais: openai-compatible (API key via env) e ollama (localhost).

* /v1/llm/complete: aceita prompt ou messages, retorna completion, usage, latency\_ms, price\_usd\_est.

* /v1/llm/bench: concorrente, repetições, sumariza latência e custo médio por (provider,model).

* Sem segredos no código: tudo via env vars; headers extras suportam env:FOO\_KEY.

* Testes: mock 100% offline; live opcional com LIVE\_LLM=1.

* Integrado ao registry com \--features modules e LLM\_PATH.

---

Quer que eu já adicione um “provider lab” pré-configurado pro teu LAB512 (ex.: Together/Mistral/DeepSeek) e crie um dashboard HTML simples em /v1/llm/bench/report.html (render no módulo) pra inspeção visual dos runs? Posso entregar isso como Mensagem 7.1 (DX) 😎

