Bora\! Mensagem \#9: invoice stub \+ persistência \+ autorização (ASK/ACK) \+ emitir cápsula 🧾🧩

Objetivo: sair do quote para um invoice stub persistido (FS/S3), controlado por permits (ASK→ACK) e com cápsula UBL opcional para auditoria.

---

# **PATCH**

## **1\) Novo módulo:** 

## **cap-invoice**

### **modules/cap-invoice/Cargo.toml**

\[package\]  
name \= "cap-invoice"  
version \= "0.1.0"  
edition \= "2021"  
license \= "MIT OR Apache-2.0"

\[features\]  
default \= \[\]  
storage-s3 \= \["ubl-storage/s3"\]  
capsule \= \["ubl\_capsule"\]

\[dependencies\]  
anyhow \= "1"  
axum \= { version \= "0.7", features \= \["json"\] }  
serde \= { version \= "1", features \= \["derive"\] }  
serde\_json \= "1"  
time \= { version \= "0.3", features \= \["macros"\] }  
uuid \= { version \= "1", features \= \["v4", "serde"\] }  
rust\_decimal \= { version \= "1", features \= \["serde"\] }  
tracing \= "0.1"  
indexmap \= "2"

cap-quote \= { path \= "../cap-quote" }  
ubl-storage \= { path \= "../../crates/ubl-storage", default-features \= false }  
ubl\_capsule \= { path \= "../../impl/rust/ubl\_capsule", optional \= true }

### **modules/cap-invoice/src/types.rs**

use serde::{Serialize, Deserialize};  
use rust\_decimal::Decimal;  
use uuid::Uuid;

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct InvoiceStub {  
    pub id: Uuid,  
    pub quote\_id: Uuid,  
    pub customer: CustomerRef,  
    pub lines: Vec\<InvoiceLine\>,  
    pub totals: Totals,  
    pub currency: String,     // "USD"/"BRL" etc.  
    pub issued\_at: String,    // RFC3339  
    pub status: InvoiceStatus,  
}

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct CustomerRef {  
    pub id: String,  
    pub name: Option\<String\>,  
    pub tax\_id: Option\<String\>, // CNPJ/CPF, VAT, etc.  
}

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct InvoiceLine {  
    pub sku: String,  
    pub qty: Decimal,  
    pub unit\_total: Decimal,  
    pub line\_total: Decimal,  
}

\#\[derive(Debug, Clone, Serialize, Deserialize, Default)\]  
pub struct Totals {  
    pub subtotal: Decimal,  
    pub tax: Decimal,  
    pub total: Decimal,  
}

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
\#\[serde(rename\_all \= "SCREAMING\_SNAKE\_CASE")\]  
pub enum InvoiceStatus { DRAFT, PENDING\_ASK, APPROVED, REJECTED }

impl Default for InvoiceStatus { fn default() \-\> Self { Self::DRAFT } }

### **modules/cap-invoice/src/store.rs**

use anyhow::Result;  
use serde::{Serialize, de::DeserializeOwned};  
use ubl\_storage::{Storage, fs::FsStore};

pub enum Store {  
    Fs(FsStore),  
    // S3(S3Store) // quando habilitar s3  
}

impl Store {  
    pub fn from\_env() \-\> Result\<Self\> {  
        if let Ok(root) \= std::env::var("INVOICE\_STORE\_DIR") {  
            Ok(Self::Fs(FsStore::new(root)))  
        } else {  
            Ok(Self::Fs(FsStore::new("./data/invoices")))  
        }  
    }

    pub async fn put\_json\<T: Serialize\>(\&self, key: \&str, v: \&T) \-\> Result\<()\> {  
        match self {  
            Store::Fs(s) \=\> {  
                let bytes \= serde\_json::to\_vec\_pretty(v)?;  
                s.put(key, bytes).await  
            }  
        }  
    }

    pub async fn get\_json\<T: DeserializeOwned\>(\&self, key: \&str) \-\> Result\<Option\<T\>\> {  
        match self {  
            Store::Fs(s) \=\> {  
                if let Some(bytes) \= s.get(key).await? {  
                    Ok(Some(serde\_json::from\_slice(\&bytes)?))  
                } else { Ok(None) }  
            }  
        }  
    }  
}

### **modules/cap-invoice/src/permit.rs**

use serde::{Serialize, Deserialize};

\#\[derive(Debug, Clone, Serialize, Deserialize)\]  
pub struct Permit {  
    pub id: String,            // ex: "invoice:issue"  
    pub subject: String,       // user/agent id  
    pub resource: String,      // invoice-id  
    pub verdict: Verdict,      // ASK/ACK/NACK  
    pub reason: Option\<String\>,  
}

\#\[derive(Debug, Clone, Serialize, Deserialize, PartialEq)\]  
\#\[serde(rename\_all \= "SCREAMING\_SNAKE\_CASE")\]  
pub enum Verdict { ASK, ACK, NACK }

impl Permit {  
    pub fn requires\_ack(\&self) \-\> bool { self.verdict \== Verdict::ASK }  
    pub fn is\_allowed(\&self) \-\> bool { self.verdict \== Verdict::ACK }  
}

### **modules/cap-invoice/src/capsule.rs**

###  **(opcional,** 

### **feature \= "capsule"**

### **)**

\#\[cfg(feature \= "capsule")\]  
use anyhow::Result;  
\#\[cfg(feature \= "capsule")\]  
use time::OffsetDateTime;  
\#\[cfg(feature \= "capsule")\]  
use ubl\_capsule::{Capsule, Seal, Bytes};

\#\[cfg(feature \= "capsule")\]  
pub fn make\_invoice\_capsule(json\_pretty: \&str, kid: \&str, sig\_bytes: \[u8;64\]) \-\> Result\<Capsule\> {  
    // env minimal: put invoice JSON as evidence URL-less (inline hash via cid)  
    let now \= OffsetDateTime::now\_utc();  
    let mut env \= serde\_json::json\!({  
        "v":"ubl-json/0.1.1",  
        "t":"record",  
        "agent":{"id":"svc:registry","name":"registry"},  
        "intent":{"kind":"ATTEST","name":"invoice-issue"},  
        "decision":{"verdict":"ACK"},  
        "ctx":{},  
        "evidence":{"urls":\[\], "cids":\[\]},  
        "meta":{"app":"ai-nrf1","tenant":"lab512","user":"system"}  
    });  
    // Criar cápsula com hdr mínimo \+ exp curto  
    let mut seal \= Seal::new\_ed25519(kid.to\_string(), Bytes::from(sig\_bytes));  
    let mut cap \= Capsule::new(env, None, \&mut seal, now.unix\_timestamp\_nanos());  
    cap.sign\_in\_place()?;  
    Ok(cap)  
}

### **modules/cap-invoice/src/lib.rs**

mod types;  
mod store;  
mod permit;  
\#\[cfg(feature="capsule")\] mod capsule;

use axum::{Json, Router, routing::{post, get}};  
use types::\*;  
use anyhow::{Result, anyhow};  
use time::OffsetDateTime;  
use uuid::Uuid;  
use tracing::info;  
use store::Store;  
use cap\_quote::api::QuoteResp;

\#\[derive(Clone)\]  
struct Ctx {  
    store: Store,  
}

pub fn router() \-\> Router {  
    let ctx \= Ctx { store: Store::from\_env().expect("store") };  
    Router::new()  
        .route("/v1/invoice/create\_from\_quote", post(create\_from\_quote))  
        .route("/v1/invoice/get/:id", get(get\_invoice))  
        .with\_state(ctx)  
}

\#\[derive(serde::Deserialize)\]  
struct CreateReq {  
    quote\_id: Uuid,  
    customer: CustomerRef,  
    currency: Option\<String\>,  
    require\_ack: Option\<bool\>,  
}

async fn create\_from\_quote(  
    axum::extract::State(ctx): axum::extract::State\<Ctx\>,  
    Json(req): Json\<CreateReq\>,  
) \-\> Result\<Json\<InvoiceStub\>, axum::http::StatusCode\> {  
    // lookup quote (em memória do cap-quote)  
    let q \= cap\_quote::get\_quote\_by\_id(req.quote\_id)  
        .ok\_or(axum::http::StatusCode::NOT\_FOUND)?;

    let issued\_at \= OffsetDateTime::now\_utc().format(\&time::format\_description::well\_known::Rfc3339).unwrap();  
    let id \= Uuid::new\_v4();  
    let lines \= q.items.iter().map(|l| InvoiceLine {  
        sku: l.sku.clone(),  
        qty: l.qty,  
        unit\_total: l.unit\_total,  
        line\_total: l.line\_total,  
    }).collect::\<Vec\<\_\>\>();

    let stub \= InvoiceStub {  
        id,  
        quote\_id: q.id,  
        customer: req.customer,  
        lines,  
        totals: Totals { subtotal: q.totals.subtotal, tax: q.totals.tax, total: q.totals.total },  
        currency: req.currency.unwrap\_or\_else(|| "USD".into()),  
        issued\_at,  
        status: if req.require\_ack.unwrap\_or(true) { InvoiceStatus::PENDING\_ASK } else { InvoiceStatus::APPROVED },  
    };

    let key \= format\!("invoices/{id}.json");  
    ctx.store.put\_json(\&key, \&stub).await.map\_err(|\_| axum::http::StatusCode::INTERNAL\_SERVER\_ERROR)?;  
    info\!(%id, "invoice created");

    Ok(Json(stub))  
}

async fn get\_invoice(  
    axum::extract::State(ctx): axum::extract::State\<Ctx\>,  
    axum::extract::Path(id): axum::extract::Path\<Uuid\>,  
) \-\> Result\<Json\<InvoiceStub\>, axum::http::StatusCode\> {  
    let key \= format\!("invoices/{id}.json");  
    let v: Option\<InvoiceStub\> \= ctx.store.get\_json(\&key).await.map\_err(|\_| axum::http::StatusCode::INTERNAL\_SERVER\_ERROR)?;  
    v.map(Json).ok\_or(axum::http::StatusCode::NOT\_FOUND)  
}

// API simples para cap-quote acessar memória  
pub fn register\_quote\_accessor() {  
    cap\_quote::expose\_read\_only(); // torna get\_quote\_by\_id() público/estável  
}  
No cap-quote/src/lib.rs, exporte:  
use uuid::Uuid;  
use super::QuoteResp;  
use once\_cell::sync::Lazy;  
use dashmap::DashMap;  
pub static QUOTES: Lazy\<DashMap\<Uuid, QuoteResp\>\> \= Lazy::new(|| DashMap::new());

pub fn get\_quote\_by\_id(id: Uuid) \-\> Option\<QuoteResp\> {  
    QUOTES.get(\&id).map(|x| x.clone())  
}  
pub fn expose\_read\_only() {}

(se já tivermos QUOTES público, mantenha só get\_quote\_by\_id)

## **2\) Registry: integrar** 

## **invoice**

##  **e** 

## **permit/ASK→ACK**

### **services/registry/Cargo.toml**

###  **(features)**

\[features\]  
modules \= \[  
  "cap-intake","cap-permit","cap-policy","cap-enrich","cap-transport","cap-llm",  
  "cap-pricing","cap-quote","cap-invoice"  
\]

\[dependencies\]  
cap-invoice \= { path \= "../../modules/cap-invoice", optional \= true }  
cap-quote   \= { path \= "../../modules/cap-quote",   optional \= true }  
cap-permit  \= { path \= "../../crates/permit",       optional \= true }

### **services/registry/src/main.rs**

###  **(mount)**

\#\[cfg(feature="modules")\]  
fn mount\_modules(router: axum::Router) \-\> axum::Router {  
    use cap\_pricing;  
    use cap\_quote;  
    use cap\_invoice;

    if let Ok(p) \= std::env::var("PRICING\_PATH") {  
        let \_ \= cap\_pricing::load\_pricing\_from(p.clone());  
        let \_ \= cap\_quote::load\_pricing(p);  
    }  
    cap\_invoice::register\_quote\_accessor();

    router  
      .merge(cap\_pricing::router())  
      .merge(cap\_quote::router())  
      .merge(cap\_invoice::router())  
}

### **(Opcional) Gate ASK/ACK antes de “emitir” (rotas de** 

### **approve/reject**

### **)**

Simples: adicionar endpoints no cap-invoice:

\#\[derive(serde::Deserialize)\]  
struct ApproveReq { id: uuid::Uuid, reason: Option\<String\> }  
\#\[derive(serde::Deserialize)\]  
struct RejectReq { id: uuid::Uuid, reason: Option\<String\> }

async fn approve\_invoice(State(ctx): State\<Ctx\>, Json(req): Json\<ApproveReq\>) \-\> Result\<Json\<InvoiceStub\>, StatusCode\> {  
    let key \= format\!("invoices/{}.json", req.id);  
    let Some(mut inv): Option\<InvoiceStub\> \= ctx.store.get\_json(\&key).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)? else {  
        return Err(StatusCode::NOT\_FOUND)  
    };  
    inv.status \= InvoiceStatus::APPROVED;  
    ctx.store.put\_json(\&key, \&inv).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
    Ok(Json(inv))  
}  
async fn reject\_invoice(State(ctx): State\<Ctx\>, Json(req): Json\<RejectReq\>) \-\> Result\<Json\<InvoiceStub\>, StatusCode\> {  
    let key \= format\!("invoices/{}.json", req.id);  
    let Some(mut inv): Option\<InvoiceStub\> \= ctx.store.get\_json(\&key).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)? else {  
        return Err(StatusCode::NOT\_FOUND)  
    };  
    inv.status \= InvoiceStatus::REJECTED;  
    ctx.store.put\_json(\&key, \&inv).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
    Ok(Json(inv))  
}

E registrar:

.route("/v1/invoice/approve", post(approve\_invoice))  
.route("/v1/invoice/reject", post(reject\_invoice))  
Em um passo seguinte podemos amarrar essas rotas ao cap-permit (policy real) exigindo token/role e auditando como receipt hop.

## **3\) Teste de integração** 

## **modules/cap-invoice/tests/flow.rs**

use axum::Router;  
use cap\_pricing;  
use cap\_quote;  
use cap\_invoice;  
use serde\_json::json;  
use tempfile::TempDir;

\#\[tokio::test\]  
async fn quote\_to\_invoice\_roundtrip() {  
    // pricing inline  
    let cfg \= r\#"  
rounding: { scale: 2, mode: half\_up }  
list: { PLAN-PRO: 49.00, ADDON-TEAM: 15.00 }  
rules: \[\]  
tax: { default\_pct: 0.00 }  
"\#;  
    let dir \= TempDir::new().unwrap();  
    std::fs::write(dir.path().join("pricing.yaml"), cfg).unwrap();  
    std::env::set\_var("PRICING\_PATH", dir.path().join("pricing.yaml"));

    // storage dir  
    std::env::set\_var("INVOICE\_STORE\_DIR", dir.path().join("store").to\_str().unwrap());

    cap\_pricing::load\_pricing\_from(std::env::var("PRICING\_PATH").unwrap()).unwrap();  
    cap\_quote::load\_pricing(std::env::var("PRICING\_PATH").unwrap()).unwrap();  
    cap\_invoice::register\_quote\_accessor();

    let app \= Router::new()  
        .merge(cap\_pricing::router())  
        .merge(cap\_quote::router())  
        .merge(cap\_invoice::router());

    let listener \= std::net::TcpListener::bind("127.0.0.1:0").unwrap();  
    let addr \= listener.local\_addr().unwrap();  
    let handle \= tokio::spawn(axum::serve(listener, app));

    // 1\) cria quote  
    let qreq \= json\!({"items":\[{"sku":"PLAN-PRO","qty":"1"}\]});  
    let q: serde\_json::Value \=  
        reqwest::Client::new().post(format\!("http://{}/v1/quote/create", addr))  
        .json(\&qreq).send().await.unwrap().json().await.unwrap();  
    let quote\_id \= q\["id"\].as\_str().unwrap().to\_string();

    // 2\) cria invoice from quote  
    let ireq \= json\!({  
        "quote\_id": quote\_id,  
        "customer": { "id":"cust-001", "name":"ACME", "tax\_id":null },  
        "currency": "USD",  
        "require\_ack": true  
    });  
    let inv: serde\_json::Value \=  
        reqwest::Client::new().post(format\!("http://{}/v1/invoice/create\_from\_quote", addr))  
        .json(\&ireq).send().await.unwrap().json().await.unwrap();  
    assert\_eq\!(inv\["status"\], "PENDING\_ASK");  
    assert\_eq\!(inv\["totals"\]\["total"\].as\_str().unwrap(), "49.00");

    // 3\) aprova (ACK)  
    let areq \= json\!({ "id": inv\["id"\].as\_str().unwrap() });  
    let inv2: serde\_json::Value \=  
        reqwest::Client::new().post(format\!("http://{}/v1/invoice/approve", addr))  
        .json(\&areq).send().await.unwrap().json().await.unwrap();  
    assert\_eq\!(inv2\["status"\], "APPROVED");

    handle.abort();  
}

## **4\) Como rodar 🔧**

\# Pricing e storage  
export PRICING\_PATH=$PWD/configs/pricing/pricing.yaml  
export INVOICE\_STORE\_DIR=$PWD/data/invoices

\# Registry com módulos  
cargo run \-p registry \--features modules \--release

### **cURL**

\# 1\) Quote  
curl \-s localhost:8790/v1/quote/create \-H 'content-type: application/json' \-d '{"items":\[{"sku":"PLAN-PRO","qty":"1"}\]}' | jq \-r '.id' \> /tmp/QID

\# 2\) Invoice (ASK)  
curl \-s localhost:8790/v1/invoice/create\_from\_quote \\  
 \-H 'content-type: application/json' \\  
 \-d '{"quote\_id":"'"$(cat /tmp/QID)"'","customer":{"id":"cust-001","name":"ACME","tax\_id":null},"currency":"USD","require\_ack":true}' | jq .

\# 3\) Approve (ACK)  
curl \-s localhost:8790/v1/invoice/approve \-H 'content-type: application/json' \\  
 \-d '{"id":"'"$(jq \-r .id \<\<\< "$(cat $INVOICE\_STORE\_DIR/../invoices/$(ls $INVOICE\_STORE\_DIR/../invoices | head \-1))")"'"}' | jq .

---

## **5\) Critérios de aceite ✅**

* Invoice stub gerado a partir de quote, com totals consistentes.

* Persistência em ubl-storage (FS por padrão; S3 quando quiser).

* Gate ASK/ACK: PENDING\_ASK → /approve → APPROVED / /reject → REJECTED.

* Endpoints REST e teste E2E.

* Opcional: geração de cápsula UBL (feature capsule) para auditoria.

---

## **6\) Próximos passos (se quiser que eu já traga na \#10) 🎯**

* Capsule completa: ubl cap sign|verify no fluxo de emissão (com Ed25519).

* Receipts encadeadas no registry a cada approve/reject (hop auditável).

* S3 (storage-s3) \+ pre-signed URLs.

* Webhook de invoice-issued com reprocessamento idempotente.

* PDF stub (wasm/print) com renderizador leve.

Quer que eu já venha com capsule \+ receipts \+ S3 na \#10? 😎

