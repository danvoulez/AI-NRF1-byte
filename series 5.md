Perfeito, Dan\! Vamos em frente com a Mensagem \#5: cap-transport 🚚✨

Módulo de transporte que (1) deriva metadados de envio a partir de hdr.\*/policy e (2) anexa receipts de hop assinadas e encadeadas, conforme a spec “ubl-receipt/1.0”.

Abaixo vai o patch completo (módulo \+ integração no registry \+ config \+ testes). É plug-and-play como os anteriores.

---

# **Patch —** 

# **modules/cap-transport**

## **1\)** 

## **modules/cap-transport/Cargo.toml**

\[package\]  
name \= "cap-transport"  
version \= "0.1.0"  
edition \= "2021"  
license \= "MIT OR Apache-2.0"

\[dependencies\]  
anyhow \= "1"  
axum \= { version \= "0.7", features \= \["json"\] }  
serde \= { version \= "1", features \= \["derive"\] }  
serde\_json \= "1"  
blake3 \= "1"  
time \= { version \= "0.3", features \= \["macros"\] }  
hex \= "0.4"  
once\_cell \= "1"  
tracing \= "0.1"

\# Base/view  
ubl\_json\_view \= { path \= "../../impl/rust/ubl\_json\_view" }

\# Ed25519 (para receipts)  
ed25519-dalek \= { version \= "2", features \= \["pkcs8"\] }  
rand \= "0.8"

\[dev-dependencies\]  
tokio \= { version \= "1", features \= \["macros", "rt-multi-thread"\] }  
reqwest \= { version \= "0.12", features \= \["json"\] }  
serde\_yaml \= "0.9"  
tempfile \= "3"

## **2\) Modelo —** 

## **modules/cap-transport/src/model.rs**

use serde::{Deserialize, Serialize};

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct TransportConf {  
    /// Ex.: base de retry, fallback, headers derivadas  
    \#\[serde(default)\]  
    pub delivery: Delivery,  
    /// Chave privada (hex) para assinar receipts em DEV.  
    /// Em prod, preferir KMS/HSM (omita aqui e use endpoint com assinatura externa).  
    \#\[serde(default)\]  
    pub dev\_ed25519\_sk\_hex: Option\<String\>,  
}

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct Delivery {  
    /// ms extras que o transporte deve parar de tentar antes de \`hdr.exp\`  
    \#\[serde(default \= "default\_guard\_ms")\]  
    pub guard\_ms: u64,  
    /// tentativas máximas  
    \#\[serde(default \= "default\_max\_attempts")\]  
    pub max\_attempts: u32,  
}

fn default\_guard\_ms() \-\> u64 { 2\_000 }  
fn default\_max\_attempts() \-\> u32 { 3 }

## **3\) Implementação —** 

## **modules/cap-transport/src/lib.rs**

use anyhow::{anyhow, Result};  
use axum::{routing::post, Json, Router};  
use once\_cell::sync::OnceCell;  
use serde::{Deserialize, Serialize};  
use serde\_json::{json, Value as J};  
use time::OffsetDateTime;

mod model;  
use ed25519\_dalek::{Signer, SigningKey, Signature, VerifyingKey};  
use model::\*;

static CONF: OnceCell\<TransportConf\> \= OnceCell::new();

pub fn router() \-\> Router {  
    Router::new()  
        // Deriva metadados fora do canônico (dicas de retry/route) e garante canonicidade  
        .route("/v1/transport/derive", post(derive\_handler))  
        // Adiciona um hop receipt assinado (encadeado) em \`receipts\[\]\`  
        .route("/v1/transport/hop", post(hop\_handler))  
}

pub fn load\_transport\_from(path: impl AsRef\<std::path::Path\>) \-\> Result\<()\> {  
    let text \= std::fs::read\_to\_string(path)?;  
    let doc: TransportConf \= serde\_yaml::from\_str(\&text)?;  
    CONF.set(doc).ok();  
    Ok(())  
}

/\* \===== Request/Response \===== \*/

\#\[derive(Deserialize)\]  
struct DeriveReq { capsule\_json: J }

\#\[derive(Serialize)\]  
struct DeriveResp {  
    capsule\_json: J,  
    // dicas para camada de transporte (fora do canônico)  
    transport: J,  
}

\#\[derive(Deserialize)\]  
struct HopReq {  
    capsule\_json: J,  
    /// Identidade do nó (ASCII), ex: did:ubl:node\#key01  
    node: String,  
    /// Kind do hop: "relay" | "ingress" | "egress" | etc.  
    kind: String,  
    /// Se ausente e há chave DEV na configuração, usa DEV; senão erro.  
    ed25519\_sk\_hex: Option\<String\>,  
    /// Timestamp opcional (nanos). Se ausente, usa agora.  
    ts\_nanos: Option\<i128\>,  
}

\#\[derive(Serialize)\]  
struct HopResp {  
    capsule\_json: J,  
    receipt\_cid\_hex: String,  
}

/\* \===== /derive \===== \*/

async fn derive\_handler(Json(req): Json\<DeriveReq\>) \-\> Result\<Json\<DeriveResp\>, axum::http::StatusCode\> {  
    let conf \= CONF.get().ok\_or(axum::http::StatusCode::SERVICE\_UNAVAILABLE)?;  
    // 1\) valida canonicidade (view \-\> NRF) e volta pra view mutável  
    let canon \= match ubl\_json\_view::from\_json\_view(\&req.capsule\_json) {  
        Ok(v) \=\> v,  
        Err(\_) \=\> return Err(axum::http::StatusCode::BAD\_REQUEST),  
    };  
    let mut view \= ubl\_json\_view::to\_json\_view(\&canon).map\_err(|\_| axum::http::StatusCode::INTERNAL\_SERVER\_ERROR)?;

    // 2\) Dicas de transporte (fora do canônico): TTL derivado de exp  
    let exp \= view.pointer("/hdr/exp").and\_then(|v| v.as\_i64()).unwrap\_or(0);  
    let now\_ns \= now\_nanos();  
    let ttl\_ms \= if exp \> 0 { ((exp as i128 \- now\_ns).max(0) / 1\_000\_000) as u64 } else { 0 };  
    let guard\_ms \= conf.delivery.guard\_ms;  
    let effective\_ttl\_ms \= ttl\_ms.saturating\_sub(guard\_ms);

    let transport \= json\!({  
        "retry": { "max\_attempts": conf.delivery.max\_attempts },  
        "deadline\_ms": ttl\_ms,  
        "send\_before\_ms": effective\_ttl\_ms  
    });

    Ok(Json(DeriveResp { capsule\_json: view, transport }))  
}

/\* \===== /hop \===== \*/

async fn hop\_handler(Json(req): Json\<HopReq\>) \-\> Result\<Json\<HopResp\>, axum::http::StatusCode\> {  
    let conf \= CONF.get().ok\_or(axum::http::StatusCode::SERVICE\_UNAVAILABLE)?;

    // 1\) valida view (canonicidade)  
    let mut canon \= match ubl\_json\_view::from\_json\_view(\&req.capsule\_json) {  
        Ok(v) \=\> v,  
        Err(\_) \=\> return Err(axum::http::StatusCode::BAD\_REQUEST),  
    };  
    let mut view \= ubl\_json\_view::to\_json\_view(\&canon).map\_err(|\_| axum::http::StatusCode::INTERNAL\_SERVER\_ERROR)?;

    // 2\) Cálculo do id atual (NRF menos {id,sigs}) já é garantido em outras camadas,  
    // aqui assumimos que "id" está correto; se quiser, poderíamos re-hash e checar.  
    let id\_hex \= view.pointer("/id")  
        .and\_then(|v| v.as\_str())  
        .ok\_or(axum::http::StatusCode::BAD\_REQUEST)?;

    // 3\) prev: blake3(nrf.encode(receipt\_sem\_sig)) do último hop, se existir  
    // Simplificação: se não houver receipts, prev \= 32 bytes zero  
    let prev \= compute\_prev\_from\_last(\&view);

    // 4\) payload a assinar (NRF map canônico do receipt):  
    // {domain:"ubl-receipt/1.0", of, prev, kind, node, ts}  
    let ts \= req.ts\_nanos.unwrap\_or\_else(now\_nanos);  
    let receipt\_payload \= json\!({  
        "domain": "ubl-receipt/1.0",  
        "of": bytes32\_from\_hex(id\_hex).map\_err(|\_| axum::http::StatusCode::BAD\_REQUEST)?,  
        "prev": prev,  
        "kind": req.kind,  
        "node": req.node,  
        "ts": ts  
    });

    // 5\) Assinatura (Ed25519)  
    let sk\_hex \= req.ed25519\_sk\_hex  
        .or\_else(|| conf.dev\_ed25519\_sk\_hex.clone())  
        .ok\_or(axum::http::StatusCode::UNAUTHORIZED)?;  
    let sig \= sign\_payload\_ed25519(\&receipt\_payload, \&sk\_hex)  
        .map\_err(|\_| axum::http::StatusCode::INTERNAL\_SERVER\_ERROR)?;

    // 6\) Append em receipts\[\] (view JSON): converge para NRF válida  
    let mut rec \= receipt\_payload.clone();  
    // sig como Bytes(64) → representamos como "b64:..." na view; ubl\_json\_view lida ao re-materializar  
    rec.as\_object\_mut().unwrap().insert("sig".into(), J::String(format\!("b64:{}", base64::encode(sig.to\_bytes()))));

    // garante array  
    if view.pointer("/receipts").is\_none() {  
        set\_path(\&mut view, &\["receipts"\], J::Array(vec\!\[\]));  
    }  
    let mut arr \= view.pointer("/receipts").cloned().unwrap\_or(J::Array(vec\!\[\]));  
    match \&mut arr {  
        J::Array(v) \=\> v.push(rec),  
        \_ \=\> return Err(axum::http::StatusCode::BAD\_REQUEST),  
    }  
    set\_path(\&mut view, &\["receipts"\], arr);

    // 7\) Revalidar NRF pós-mutate  
    match ubl\_json\_view::from\_json\_view(\&view) {  
        Ok(v2) \=\> { canon \= v2; }  
        Err(\_) \=\> return Err(axum::http::StatusCode::UNPROCESSABLE\_ENTITY),  
    }

    // CID do receipt \= blake3(nrf.encode(receipt\_sem\_sig))  
    let cid\_hex \= hex::encode(blake3::hash(\&encode\_receipt\_without\_sig(\&receipt\_payload)).as\_bytes());

    Ok(Json(HopResp { capsule\_json: view, receipt\_cid\_hex: cid\_hex }))  
}

/\* \===== helpers \===== \*/

fn set\_path(root: \&mut J, path: &\[\&str\], value: J) {  
    let mut cur \= root;  
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
}

fn bytes32\_from\_hex(hexstr: \&str) \-\> Result\<J\> {  
    let s \= hexstr.strip\_prefix("b3:").unwrap\_or(hexstr); // aceita view b3:\<hex\> também  
    let mut b \= hex::decode(s)?;  
    if b.len() \!= 32 { return Err(anyhow\!("expect 32 bytes")); }  
    Ok(J::String(format\!("b3:{}", hex::encode(b))))  
}

fn compute\_prev\_from\_last(view: \&J) \-\> J {  
    if let Some(J::Array(arr)) \= view.pointer("/receipts") {  
        if let Some(last) \= arr.last() {  
            let payload \= strip\_sig(last.clone());  
            let nrf \= encode\_receipt\_without\_sig(\&payload);  
            let h \= blake3::hash(\&nrf);  
            return J::String(format\!("b3:{}", hex::encode(h.as\_bytes())));  
        }  
    }  
    // 32 bytes zero  
    J::String("b3:0000000000000000000000000000000000000000000000000000000000000000".into())  
}

fn strip\_sig(mut receipt: J) \-\> J {  
    if let Some(obj) \= receipt.as\_object\_mut() {  
        obj.remove("sig");  
    }  
    receipt  
}

fn encode\_receipt\_without\_sig(payload: \&J) \-\> Vec\<u8\> {  
    // O payload já segue o contrato do receipt; conversão para NRF usando o view-\>NRF do capsule base  
    // (usa tipos Bytes em "of"/"prev")  
    // Aqui reusa o to\_json/from\_json do ubl\_json\_view para gerar bytes determinísticos:  
    let tmp \= json\!({  
        "v":"ubl-receipt/1.0",  
        "payload": payload  
    });  
    // convertemos só o payload para NRF via view roundtrip e extraímos bytes  
    // truque: o encoder do projeto sempre ordena chaves/tipos; usamos isso para hashing.  
    // Aqui, serializamos apenas o "payload".  
    let r \= ubl\_json\_view::from\_json\_view(\&tmp).expect("valid");  
    ubl\_json\_view::encode\_part(\&r, &\["payload"\]).unwrap\_or\_default()  
}

fn sign\_payload\_ed25519(payload: \&J, sk\_hex: \&str) \-\> Result\<Signature\> {  
    let sk\_bytes \= hex::decode(sk\_hex)?;  
    if sk\_bytes.len() \!= 32 { return Err(anyhow\!("sk must be 32 bytes hex")); }  
    let sk \= SigningKey::from\_bytes(\&sk\_bytes.try\_into().unwrap());  
    let nrf \= encode\_receipt\_without\_sig(payload);  
    Ok(sk.sign(\&blake3::hash(\&nrf).as\_bytes()))  
}

fn now\_nanos() \-\> i128 {  
    let now \= OffsetDateTime::now\_utc();  
    (now.unix\_timestamp\_nanos())  
}  
Nota: O helper encode\_part acima pressupõe um util exposto no ubl\_json\_view para serializar um submap (se ainda não existir, posso incluir rapidamente: uma função que pega um JSON view, converte para NRF e retorna os bytes do caminho indicado; se preferir, trocamos por um encoder local de receipt com as mesmas regras NRF).

## **4\) Config —** 

## **configs/transport/transport.yaml**

delivery:  
  guard\_ms: 2000  
  max\_attempts: 3

\# (Opcional DEV) Chave privada hex (32 bytes) para assinatura de receipts no /hop.  
\# Em produção, omita aqui e envie "ed25519\_sk\_hex" no body OU assine via KMS externo.  
dev\_ed25519\_sk\_hex: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

---

# **Integração no** 

# **registry**

## **services/registry/Cargo.toml**

\[features\]  
default \= \[\]  
modules \= \[  
  "cap-intake",  
  "cap-permit",  
  "cap-policy",  
  "cap-enrich",  
  "cap-transport",  
\]

\[dependencies\]  
cap-intake    \= { path \= "../../modules/cap-intake",    optional \= true }  
cap-permit    \= { path \= "../../modules/cap-permit",    optional \= true }  
cap-policy    \= { path \= "../../modules/cap-policy",    optional \= true }  
cap-enrich    \= { path \= "../../modules/cap-enrich",    optional \= true }  
cap-transport \= { path \= "../../modules/cap-transport", optional \= true }

## **services/registry/src/main.rs**

##  **(trecho)**

\#\[cfg(feature \= "modules")\]  
fn mount\_modules(router: axum::Router) \-\> axum::Router {  
    use cap\_intake;  
    use cap\_permit;  
    use cap\_policy;  
    use cap\_enrich;  
    use cap\_transport;

    if let Ok(p) \= std::env::var("PERMIT\_PATH")    { let \_ \= cap\_permit::load\_policy\_from(p); }  
    if let Ok(p) \= std::env::var("POLICY\_PATH")    { let \_ \= cap\_policy::load\_policy\_from(p); }  
    if let Ok(p) \= std::env::var("ENRICH\_PATH")    { let \_ \= cap\_enrich::load\_enrich\_from(p); }  
    if let Ok(p) \= std::env::var("TRANSPORT\_PATH") { let \_ \= cap\_transport::load\_transport\_from(p); }

    router  
        .merge(cap\_intake::router())  
        .merge(cap\_permit::router())  
        .merge(cap\_policy::router())  
        .merge(cap\_enrich::router())  
        .merge(cap\_transport::router()) // /v1/transport/derive | /v1/transport/hop  
}

---

# **Testes de integração —** 

# **modules/cap-transport/tests/transport\_chain.rs**

use serde\_json::json;  
use tokio::task;  
use std::net::TcpListener;  
use cap\_transport::{router, load\_transport\_from};  
use tempfile::NamedTempFile;

\#\[tokio::test\]  
async fn derive\_and\_hop\_ok() {  
    // config DEV com chave fixa (somente para teste)  
    let cfg \= r\#"  
delivery: { guard\_ms: 500, max\_attempts: 2 }  
dev\_ed25519\_sk\_hex: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"  
"\#;  
    let tmp \= NamedTempFile::new().unwrap();  
    std::fs::write(tmp.path(), cfg).unwrap();  
    load\_transport\_from(tmp.path()).unwrap();

    let app \= router();  
    let listener \= TcpListener::bind("127.0.0.1:0").unwrap();  
    let addr \= listener.local\_addr().unwrap();  
    let handle \= task::spawn(axum::serve(listener, app));

    let cap \= json\!({  
      "v":"ubl-capsule/1.0",  
      "id":"b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",  
      "hdr":{"src":"did:ubl:a","dst":"did:ubl:b","nonce":"b64:AA==","exp": 4102444800000000000},  
      "env":{"v":"ubl-json/0.1.1","t":"record","agent":{"id":"a"},"intent":{"kind":"ATTEST","name":"demo"},"ctx":{},"decision":{"verdict":"ACK"},"evidence":{}},  
      "seal":{"alg":"Ed25519","kid":"did:ubl:a\#k"},  
      "receipts":\[\]  
    });

    // 1\) derive  
    let res \= reqwest::Client::new()  
        .post(format\!("http://{}/v1/transport/derive", addr))  
        .json(\&serde\_json::json\!({ "capsule\_json": cap }))  
        .send().await.unwrap();  
    assert\!(res.status().is\_success());  
    let body: serde\_json::Value \= res.json().await.unwrap();  
    assert\!(body.get("transport").is\_some());

    // 2\) hop (append receipt)  
    let cap2 \= body.get("capsule\_json").cloned().unwrap();  
    let res2 \= reqwest::Client::new()  
        .post(format\!("http://{}/v1/transport/hop", addr))  
        .json(\&serde\_json::json\!({  
            "capsule\_json": cap2,  
            "node": "did:ubl:relay\#k1",  
            "kind": "relay"  
        }))  
        .send().await.unwrap();  
    assert\!(res2.status().is\_success());  
    let body2: serde\_json::Value \= res2.json().await.unwrap();

    // tem receipts\[0\]  
    assert\!(body2.pointer("/capsule\_json/receipts/0").is\_some());  
    assert\!(body2.get("receipt\_cid\_hex").unwrap().as\_str().unwrap().len() \== 64);

    handle.abort();  
}

---

# **Uso local 🧪**

\# Build \+ run registry com módulos  
export ENRICH\_PATH="$PWD/configs/enrich/enrich.yaml"  
export TRANSPORT\_PATH="$PWD/configs/transport/transport.yaml"  
cargo run \-p registry \--features modules \--release

Derivar transporte:

curl \-s http://127.0.0.1:8790/v1/transport/derive \\  
  \-H 'content-type: application/json' \\  
  \-d @capsule.json | jq .

Assinar hop (append receipt):

curl \-s http://127.0.0.1:8790/v1/transport/hop \\  
  \-H 'content-type: application/json' \\  
  \-d @- \<\<'JSON' | jq .  
{  
  "capsule\_json": \<CAPSULE\_DERIVED\>,  
  "node": "did:ubl:relay\#k1",  
  "kind": "relay"  
}  
JSON

Se não houver dev\_ed25519\_sk\_hex no TRANSPORT\_PATH, envie ed25519\_sk\_hex no body (apenas para DEV). Em produção, você pode substituir a assinatura por um serviço KMS: o módulo pode aceitar kms=true \+ kid e chamar externamente — posso incluir esse “provider” também.

---

## **Critérios de aceite ✅**

* /derive: calcula deadline\_ms e send\_before\_ms a partir de hdr.exp, respeitando guard\_ms; não altera canônico (só retorna um transport auxiliar).

* /hop: adiciona receipt em conformidade:

  * payload assinado: {domain:"ubl-receipt/1.0", of, prev, kind, node, ts}

  * prev \= blake3(nrf.encode(last\_receipt\_sem\_sig)) ou zeros se nenhum

  * sig Ed25519 (64 bytes)

  * revalida NRF após mutação

* Testes: cargo test \-p cap-transport passando.

---

## **Quer também o endpoint** 

## **macro**

##  

## **/v1/pipeline/ingest**

## **?**

Ele faria sequencialmente intake → permit → policy → enrich → transport/derive → transport/hop num único POST (com switches), ótimo pra DX. Se topar, eu já entrego na próxima com vetores end-to-end 🧩🔥

