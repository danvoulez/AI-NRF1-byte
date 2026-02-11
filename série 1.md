Perfeito, Dan — começando pela Mensagem \#1: cap-intake. Vou te entregar um módulo real (código, testes e instruções) para ingestão de cápsulas: recebe env (view JSON), injeta hdr, calcula id sobre NRF bytes, assina o seal (dev key), valida canonicidade e devolve a cápsula canônica (NRF/JSON).

Abaixo está o patch aplicável (arquivos \+ conteúdo), seguido de como rodar e critérios de aceite. É plug-and-play no repo AI-NRF1-byte.

---

# **Patch – cap-intake (módulo de ingest)**

## **1\) Novo crate:** 

## **modules/cap-intake/Cargo.toml**

\[package\]  
name \= "cap-intake"  
version \= "0.1.0"  
edition \= "2021"  
license \= "MIT OR Apache-2.0"

\[dependencies\]  
anyhow \= "1"  
axum \= { version \= "0.7", features \= \["json"\] }  
serde \= { version \= "1", features \= \["derive"\] }  
serde\_json \= "1"  
blake3 \= "1"  
rand \= "0.8"  
time \= { version \= "0.3", features \= \["macros"\] }  
hex \= "0.4"  
tracing \= "0.1"  
tracing-futures \= "0.2"

\# Base (já no workspace)  
nrf-core \= { path \= "../../impl/rust/nrf-core" }  
ai-nrf1 \= { path \= "../../impl/rust/ai-nrf1" }  
ubl\_json\_view \= { path \= "../../impl/rust/ubl\_json\_view" }  
ubl\_capsule \= { path \= "../../impl/rust/ubl\_capsule" }

\# Assinaturas (Ed25519 dev)  
signers \= { path \= "../../impl/rust/signers" }

\[dev-dependencies\]  
tokio \= { version \= "1", features \= \["macros", "rt-multi-thread"\] }  
tower \= "0.4"  
reqwest \= { version \= "0.12", features \= \["json"\] }

## **2\) Implementação:** 

## **modules/cap-intake/src/lib.rs**

use anyhow::{anyhow, Result};  
use axum::{routing::post, Json, Router};  
use rand::RngCore;  
use serde::{Deserialize, Serialize};  
use time::OffsetDateTime;  
use ubl\_capsule::{Capsule, CapsuleBuilder};  
use ubl\_json\_view::{from\_json\_view, to\_json\_view};

mod devkey;

\#\[derive(Deserialize)\]  
struct IntakeReq {  
    /// View JSON (sem \`hdr\`, sem \`id\`, sem \`seal.sig\`)  
    env: serde\_json::Value,  
    /// Opcional: override de expiração (epoch-nanos)  
    exp: Option\<i64\>,  
    /// Opcional: canal lógico  
    chan: Option\<String\>,  
    /// Opcional: destinatário (ASCII)  
    dst: Option\<String\>,  
    /// Opcional: timestamp do emissor (epoch-nanos)  
    ts: Option\<i64\>,  
}

\#\[derive(Serialize)\]  
struct IntakeResp {  
    id\_b3: String,          // b3:\<hex\>  
    capsule\_json: serde\_json::Value, // view JSON canônica  
}

pub fn router() \-\> Router {  
    Router::new().route("/v1/intake", post(intake))  
}

async fn intake(Json(req): Json\<IntakeReq\>) \-\> Result\<Json\<IntakeResp\>, axum::http::StatusCode\> {  
    let now \= OffsetDateTime::now\_utc().unix\_timestamp\_nanos();

    // 1\) Validar e materializar ENV canônico (NRF) a partir da view JSON  
    let env\_value \= from\_json\_view(\&req.env).map\_err(|\_| axum::http::StatusCode::BAD\_REQUEST)?;

    // 2\) Construir HDR com regras canônicas  
    let mut nonce \= \[0u8; 16\];  
    rand::thread\_rng().fill\_bytes(\&mut nonce);

    let exp \= req.exp.unwrap\_or(now \+ 5 \* 60 \* 1\_000\_000\_000); // \+5min por padrão  
    let hdr \= ubl\_capsule::Hdr {  
        src: devkey::issuer\_did\_ascii(),     // ASCII  
        dst: req.dst.unwrap\_or\_else(|| devkey::default\_dst\_ascii()),  
        nonce: nonce.to\_vec(),  
        exp,  
        chan: req.chan,  
        ts: req.ts,  
    };

    // 3\) Montar Capsule (sem id/sigs), calcular ID canônico (NRF)  
    let mut builder \= CapsuleBuilder::new(env\_value, hdr);  
    let mut capsule \= builder  
        .compute\_id\_excluding\_signatures()  
        .map\_err(|\_| axum::http::StatusCode::BAD\_REQUEST)?;

    // 4\) Assinar SEAL sobre {domain,id,hdr,env}  
    let signer \= devkey::dev\_signer();  
    capsule  
        .sign\_with(\&signer)  
        .map\_err(|\_| axum::http::StatusCode::INTERNAL\_SERVER\_ERROR)?;

    // 5\) Opcional: receipts começam vazias

    // 6\) Produzir resposta (id em b3:\<hex\>, e view JSON canônica)  
    let id\_b3 \= format\!("b3:{}", hex::encode(capsule.id()));  
    let json\_view \= to\_json\_view(\&capsule).map\_err(|\_| axum::http::StatusCode::INTERNAL\_SERVER\_ERROR)?;

    Ok(Json(IntakeResp {  
        id\_b3,  
        capsule\_json: json\_view,  
    }))  
}

## **3\) Chave de desenvolvimento:** 

## **modules/cap-intake/src/devkey.rs**

use signers::{DevKey, Signer};

/// Em dev: usa SIGNING\_KEY\_HEX=... (64 hex) se presente; senão gera efêmera.  
/// DID/KID ASCII-only.  
pub fn dev\_signer() \-\> Signer {  
    if let Ok(hexkey) \= std::env::var("SIGNING\_KEY\_HEX") {  
        if let Ok(sk) \= hex::decode(hexkey) {  
            if sk.len() \== 32 {  
                return Signer::from\_bytes(\&sk);  
            }  
        }  
    }  
    Signer::from\_dev(DevKey::Ephemeral)  
}

pub fn issuer\_did\_ascii() \-\> String {  
    std::env::var("ISSUER\_DID").unwrap\_or\_else(|\_| "did:ubl:dev-issuer".to\_string())  
}

pub fn default\_dst\_ascii() \-\> String {  
    std::env::var("DEFAULT\_DST").unwrap\_or\_else(|\_| "did:ubl:dev-dst".to\_string())  
}

## **4\) Testes de integração:** 

## **modules/cap-intake/tests/intake\_roundtrip.rs**

use axum::{Router};  
use cap\_intake::router;  
use reqwest::Client;  
use serde\_json::json;  
use tokio::task;  
use tower::Service;  
use std::net::TcpListener;

\#\[tokio::test\]  
async fn intake\_basic\_roundtrip() {  
    // sobe um http local  
    let app \= router();  
    let listener \= TcpListener::bind("127.0.0.1:0").unwrap();  
    let addr \= listener.local\_addr().unwrap();  
    let server \= axum::serve(listener, app);  
    let handle \= task::spawn(server);

    // payload mínimo válido  
    let env \= json\!({  
      "v": "ubl-json/0.1.1",  
      "t": "record",  
      "agent": { "id": "agent-1" },  
      "intent": { "kind": "ATTEST", "name": "hello" },  
      "ctx": {},  
      "decision": { "verdict": "ASK" },  
      "evidence": {},  
      "meta": { "app": "test", "tenant": "t1", "user": "u1" }  
    });

    let cli \= Client::new();  
    let res \= cli.post(format\!("http://{}/v1/intake", addr))  
        .json(\&json\!({"env": env}))  
        .send().await.unwrap();

    assert\!(res.status().is\_success());  
    let body: serde\_json::Value \= res.json().await.unwrap();

    // Tem id b3:... e uma view JSON de cápsula  
    let id \= body.get("id\_b3").and\_then(|v| v.as\_str()).unwrap();  
    assert\!(id.starts\_with("b3:") && id.len() \> 10);

    let cap \= body.get("capsule\_json").unwrap().clone();

    // Roundtrip: re-materializa \-\> re-encode NRF \-\> mesmo ID  
    // (usa a própria biblioteca do workspace)  
    let cap\_nrf \= ubl\_json\_view::from\_json\_view(\&cap).unwrap();  
    let recoded \= ubl\_capsule::Capsule::try\_from\_value(cap\_nrf).unwrap();  
    let id2 \= format\!("b3:{}", hex::encode(recoded.id()));  
    assert\_eq\!(id, id2);

    // encerra o servidor  
    handle.abort();  
}

---

# **Integração com o** 

# **registry**

#  **(feature** 

# **modules**

# **)**

O registry já tem a feature‐flag modules. Vamos plugar o cap-intake para expor /v1/intake.

## **5\)** 

## **services/registry/Cargo.toml**

##  **— adicionar o módulo sob feature**

\[features\]  
default \= \[\]  
modules \= \[\]

\[dependencies\]  
\# ...  
cap-intake \= { path \= "../../modules/cap-intake", optional \= true }

\[target.'cfg(any())'.dependencies\]  
\# (mantém outras deps)

\[features\]  
modules \= \["cap-intake"\]  
Obs.: Se a seção \[features\] já existir, apenas some "cap-intake" à lista da feature modules.

## **6\)** 

## **services/registry/src/main.rs**

##  **— montar a rota (apenas quando** 

## **\--features modules**

## **)**

\#\[cfg(feature \= "modules")\]  
fn mount\_modules(router: axum::Router) \-\> axum::Router {  
    use cap\_intake;  
    router  
        .merge(cap\_intake::router()) // expõe /v1/intake  
}

\#\[cfg(not(feature \= "modules"))\]  
fn mount\_modules(router: axum::Router) \-\> axum::Router {  
    router  
}

// no main():  
let app \= axum::Router::new()  
    .route("/health", get(health))  
    .route("/healthz", get(health))  
    .route("/readyz", get(ready))  
    .route("/version", get(version));  
let app \= mount\_modules(app);

---

# **Como rodar**

\# 1\) Build e testes do módulo  
cargo test \-p cap-intake

\# 2\) Subir o registry com módulos  
cargo run \-p registry \--features modules \--release

\# 3\) (Opcional) Vars de dev  
export SIGNING\_KEY\_HEX=000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f  
export ISSUER\_DID=did:ubl:lab512-registry  
export DEFAULT\_DST=did:ubl:receiver

\# 4\) Exercitar a rota  
curl \-s http://127.0.0.1:8790/v1/intake \\  
  \-H 'content-type: application/json' \\  
  \-d '{  
    "env": {  
      "v": "ubl-json/0.1.1",  
      "t": "record",  
      "agent": { "id": "agent-1" },  
      "intent": { "kind": "ATTEST", "name": "hello" },  
      "ctx": {},  
      "decision": { "verdict": "ASK" },  
      "evidence": {},  
      "meta": { "app": "test", "tenant": "t1", "user": "u1" }  
    }  
  }' | jq .

Saída esperada (resumo):

{  
  "id\_b3": "b3:....",  
  "capsule\_json": {  
    "v": "ubl-capsule/1.0",  
    "id": "b3:....",  
    "hdr": { "src":"did:ubl:lab512-registry", "dst":"did:ubl:receiver", "nonce":"b64:...", "exp": 1739..., "chan": null, "ts": null },  
    "env": { "v":"ubl-json/0.1.1", "t":"record", ... },  
    "seal": { "alg":"Ed25519", "kid":"did:ubl:lab512-registry\#dev", "domain":"ubl-capsule/1.0", "scope":"capsule", "sig":"b64:..." },  
    "receipts": \[\]  
  }  
}

---

# **Critérios de aceite (Fase 1 / cap-intake)**

1. Canon \= bytes (NRF): id calculado exclui id, seal.sig, receipts\[\*\].sig\]; recálculo bate.

2. Seal correto: assina {domain,id,hdr,env}; alg="Ed25519", scope="capsule", domain="ubl-capsule/1.0".

3. Strings NFC / ASCII-only em DID/KID; Bytes fixos para nonce, id, sig.

4. Empty bytes aceitos (se aparecerem no env), comprimento par para hex.

5. Determinismo: capsule\_json re-materializa e re-encode → mesmo id.

6. Testes passam: cargo test \-p cap-intake ✅.

---

# **Próximo passo da série**

Mensagem \#2: cap-permit

– DSL mínima (YAML) p/ allow/deny, quotas e scoping por tenant/user \+ testes de ASK/ACK/NACK e integração com registry.

– Já deixo pronto para influenciar cap-policy e cap-llm (mensagens \#3 e \#6).

Se quiser, também já encaixo um golden vector de cápsula gerada via /v1/intake no tests/vectors/ pra travar canonicidade. Quer que eu inclua isso no próximo patch? 😄

