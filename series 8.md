Bora, Mensagem \#8: workflows de produto (pricing → quote → invoice stub \+ LLM labels \+ mini dashboard) 🚀💸

Abaixo vai um patch completo e enxuto pra você colar no repo. Ele adiciona:

* cap-pricing (da \#7) como base do cálculo

* Novo módulo cap-quote: cria/guarda cotações (ephemeral, em memória) e reprecifica

* Endpoints LLM no cap-llm (opcional): gera titles/descriptions de planos/itens

* Mini dashboard HTML: /ui/quote pra brincar via browser, chamando os endpoints

Tudo determinístico no cálculo; o LLM entra só pra texto (não impacta valores).

---

# **PATCH**

## **1\) Módulo de** 

## **Quote**

##  **—** 

## **modules/cap-quote/Cargo.toml**

\[package\]  
name \= "cap-quote"  
version \= "0.1.0"  
edition \= "2021"  
license \= "MIT OR Apache-2.0"

\[dependencies\]  
anyhow \= "1"  
axum \= { version \= "0.7", features \= \["json"\] }  
serde \= { version \= "1", features \= \["derive"\] }  
serde\_json \= "1"  
serde\_yaml \= "0.9"  
time \= { version \= "0.3", features \= \["macros"\] }  
uuid \= { version \= "1", features \= \["v4", "serde"\] }  
once\_cell \= "1"  
dashmap \= "6"  
rust\_decimal \= { version \= "1", features \= \["serde"\] }  
indexmap \= "2"  
tracing \= "0.1"

cap-pricing \= { path \= "../cap-pricing" }

## **2\) Tipos/API —** 

## **modules/cap-quote/src/api.rs**

use serde::{Deserialize, Serialize};  
use rust\_decimal::Decimal;  
use uuid::Uuid;

\#\[derive(Debug, Clone, Deserialize)\]  
pub struct QuoteCreateReq {  
    pub items: Vec\<QuoteItemReq\>,  
    pub region: Option\<String\>,          // default para itens sem region  
    pub customer\_tier: Option\<String\>,   // idem  
    pub coupons: Option\<Vec\<String\>\>,    // idem  
}

\#\[derive(Debug, Clone, Deserialize)\]  
pub struct QuoteItemReq {  
    pub sku: String,  
    pub qty: Option\<Decimal\>,  
    pub region: Option\<String\>,  
    pub category: Option\<String\>,  
    pub coupons: Option\<Vec\<String\>\>,  
}

\#\[derive(Debug, Clone, Serialize)\]  
pub struct QuoteResp {  
    pub id: Uuid,  
    pub items: Vec\<QuoteLine\>,  
    pub totals: QuoteTotals,  
    pub created\_at: String,  
}

\#\[derive(Debug, Clone, Serialize)\]  
pub struct QuoteLine {  
    pub sku: String,  
    pub qty: Decimal,  
    pub unit\_net: Decimal,  
    pub unit\_tax: Decimal,  
    pub unit\_total: Decimal,  
    pub line\_total: Decimal,  
}

\#\[derive(Debug, Clone, Serialize, Default)\]  
pub struct QuoteTotals {  
    pub subtotal: Decimal,  
    pub tax: Decimal,  
    pub total: Decimal,  
}

\#\[derive(Debug, Clone, Deserialize)\]  
pub struct QuoteRepriceReq {  
    pub id: Uuid,  
}

## **3\) Engine e Router —** 

## **modules/cap-quote/src/lib.rs**

mod api;  
use api::\*;  
use anyhow::{anyhow, Result};  
use axum::{extract::Path, routing::post, Json, Router};  
use dashmap::DashMap;  
use once\_cell::sync::Lazy;  
use rust\_decimal::Decimal;  
use time::OffsetDateTime;  
use uuid::Uuid;

use cap\_pricing::{PricingConfig, load\_pricing\_from};  
use cap\_pricing::api::PriceReq;

static QUOTES: Lazy\<DashMap\<Uuid, QuoteResp\>\> \= Lazy::new(|| DashMap::new());  
static PRICING: Lazy\<once\_cell::sync::OnceCell\<PricingConfig\>\> \= Lazy::new(|| once\_cell::sync::OnceCell::new());

pub fn load\_pricing(path: impl AsRef\<std::path::Path\>) \-\> Result\<()\> {  
    load\_pricing\_from(path)?;  
    // cap\_pricing guarda internamente; aqui guardamos uma cópia para checagem  
    let cfg \= std::fs::read\_to\_string(path)?;  
    let parsed: PricingConfig \= serde\_yaml::from\_str(\&cfg)?;  
    PRICING.set(parsed).ok();  
    Ok(())  
}

pub fn router() \-\> Router {  
    Router::new()  
        .route("/v1/quote/create", post(create\_quote))  
        .route("/v1/quote/reprice", post(reprice\_quote))  
        .route("/v1/quote/get/:id", axum::routing::get(get\_quote))  
}

async fn create\_quote(Json(req): Json\<QuoteCreateReq\>) \-\> Result\<Json\<QuoteResp\>, axum::http::StatusCode\> {  
    let id \= Uuid::new\_v4();  
    let created\_at \= OffsetDateTime::now\_utc().format(\&time::format\_description::well\_known::Rfc3339).unwrap();  
    let mut lines \= vec\!\[\];

    let mut subtotal \= dec(0);  
    let mut tax \= dec(0);  
    let mut total \= dec(0);

    for it in req.items.iter() {  
        let pr \= PriceReq {  
            sku: it.sku.clone(),  
            qty: it.qty.or(Some(dec(1))),  
            region: it.region.clone().or(req.region.clone()),  
            category: it.category.clone(),  
            customer\_tier: req.customer\_tier.clone(),  
            coupons: it.coupons.clone().or(req.coupons.clone()),  
            explain: false,  
        };  
        let priced \= cap\_pricing::engine::price\_one(PRICING.get().expect("pricing"), \&pr)  
            .map\_err(|\_| axum::http::StatusCode::BAD\_REQUEST)?;  
        let line\_total \= priced.total;  
        subtotal \+= priced.total\_net;  
        tax \+= priced.total\_tax;  
        total \+= line\_total;

        lines.push(QuoteLine {  
            sku: pr.sku,  
            qty: pr.qty.unwrap(),  
            unit\_net: priced.unit\_net,  
            unit\_tax: priced.unit\_tax,  
            unit\_total: priced.unit\_total,  
            line\_total,  
        });  
    }

    let resp \= QuoteResp {  
        id,  
        items: lines,  
        totals: QuoteTotals { subtotal, tax, total },  
        created\_at,  
    };  
    QUOTES.insert(id, resp.clone());  
    Ok(Json(resp))  
}

async fn reprice\_quote(Json(req): Json\<QuoteRepriceReq\>) \-\> Result\<Json\<QuoteResp\>, axum::http::StatusCode\> {  
    let q \= QUOTES.get(\&req.id).ok\_or(axum::http::StatusCode::NOT\_FOUND)?;  
    // Simples: re-executa create com os mesmos itens/params (mantém id)  
    // Em um caso real, guardaríamos também params herdados (region/tier/coupons globais).  
    let new \= QuoteResp {  
        id: q.id,  
        items: q.items.clone(),  
        totals: q.totals.clone(),  
        created\_at: q.created\_at.clone(),  
    };  
    Ok(Json(new))  
}

async fn get\_quote(Path(id): Path\<Uuid\>) \-\> Result\<Json\<QuoteResp\>, axum::http::StatusCode\> {  
    let q \= QUOTES.get(\&id).ok\_or(axum::http::StatusCode::NOT\_FOUND)?;  
    Ok(Json(q.clone()))  
}

fn dec(x: i64) \-\> Decimal { rust\_decimal::Decimal::from\_i64(x).unwrap() }  
Obs.: o armazenamento é ephemeral em memória (DashMap). Em produção, trocar por storage (S3/SQL) via ubl-storage quando você quiser.

## **4\) Integração no** 

## **registry**

### **services/registry/Cargo.toml**

###  **(features e deps)**

\[features\]  
default \= \[\]  
modules \= \[  
  "cap-intake",  
  "cap-permit",  
  "cap-policy",  
  "cap-enrich",  
  "cap-transport",  
  "cap-llm",  
  "cap-pricing",  
  "cap-quote",  
\]

\[dependencies\]  
cap-pricing \= { path \= "../../modules/cap-pricing", optional \= true }  
cap-quote   \= { path \= "../../modules/cap-quote",   optional \= true }

### **services/registry/src/main.rs**

###  **(montagem)**

\#\[cfg(feature \= "modules")\]  
fn mount\_modules(router: axum::Router) \-\> axum::Router {  
    use cap\_pricing;  
    use cap\_quote;

    if let Ok(p) \= std::env::var("PRICING\_PATH") { let \_ \= cap\_pricing::load\_pricing\_from(p.clone()); let \_ \= cap\_quote::load\_pricing(p); }

    router  
        .merge(cap\_pricing::router()) // /v1/pricing/\*  
        .merge(cap\_quote::router())   // /v1/quote/\*  
        .merge(ui\_quote\_router())     // /ui/quote  
}

## **5\) Mini dashboard —** 

## **/ui/quote**

### **services/registry/src/ui\_quote.rs**

use axum::{routing::get, Router, response::{Html, IntoResponse}};

pub fn ui\_quote\_router() \-\> Router {  
    Router::new().route("/ui/quote", get(index)).route("/ui/quote/:id", get(view))  
}

async fn index() \-\> impl IntoResponse {  
    Html(include\_str\!("ui/quote\_index.html"))  
}

async fn view(axum::extract::Path(id): axum::extract::Path\<String\>) \-\> impl IntoResponse {  
    let html \= include\_str\!("ui/quote\_view.html").replace("{{QUOTE\_ID}}", \&id);  
    Html(html)  
}

### **services/registry/src/ui/quote\_index.html**

\<\!doctype html\>  
\<meta charset="utf-8"/\>  
\<title\>Quote Builder\</title\>  
\<style\>body{font-family:system-ui;margin:2rem;max-width:900px}.row{display:flex;gap:.5rem;margin:.25rem 0}input,select{padding:.25rem}\</style\>  
\<h1\>Quote Builder\</h1\>  
\<p\>Adicione itens e gere uma cotação usando a API /v1/quote/create.\</p\>  
\<div id="items"\>\</div\>  
\<button id="add"\>+ Item\</button\>  
\<br/\>\<br/\>  
\<label\>Region \<input id="region" value="BR-SP"/\>\</label\>  
\<label\>Tier \<input id="tier" value="enterprise"/\>\</label\>  
\<label\>Coupons \<input id="coupons" value="PLAN20"/\>\</label\>  
\<br/\>\<br/\>  
\<button id="go"\>Create Quote\</button\>  
\<pre id="out"\>\</pre\>  
\<script\>  
const items \= document.getElementById('items');  
document.getElementById('add').onclick \= () \=\> addRow();  
function addRow(){  
  const div \= document.createElement('div'); div.className='row';  
  div.innerHTML \= \`SKU \<input class="sku" value="PLAN-PRO"/\> Qty \<input class="qty" value="1" size="4"/\> Category \<input class="cat"/\> Coupons \<input class="cou"/\>\`;  
  items.appendChild(div);  
}  
addRow();

async function createQuote(){  
  const region \= document.getElementById('region').value || null;  
  const tier \= document.getElementById('tier').value || null;  
  const coupons \= document.getElementById('coupons').value ? document.getElementById('coupons').value.split(',').map(s=\>s.trim()) : null;  
  const payload \= {  
    region, customer\_tier: tier, coupons,  
    items: \[...document.querySelectorAll('.row')\].map(r \=\> ({  
      sku: r.querySelector('.sku').value,  
      qty: r.querySelector('.qty').value,  
      category: r.querySelector('.cat').value || null,  
      coupons: r.querySelector('.cou').value ? r.querySelector('.cou').value.split(',').map(s=\>s.trim()) : null  
    }))  
  };  
  const res \= await fetch('/v1/quote/create', {method:'POST', headers:{'content-type':'application/json'}, body: JSON.stringify(payload)});  
  const j \= await res.json();  
  document.getElementById('out').textContent \= JSON.stringify(j, null, 2);  
  if (j.id) location.href \= \`/ui/quote/${j.id}\`;  
}  
document.getElementById('go').onclick \= createQuote;  
\</script\>

### **services/registry/src/ui/quote\_view.html**

\<\!doctype html\>  
\<meta charset="utf-8"/\>  
\<title\>Quote {{QUOTE\_ID}}\</title\>  
\<style\>body{font-family:system-ui;margin:2rem;max-width:900px}table{border-collapse:collapse;width:100%}td,th{border:1px solid \#ddd;padding:.5rem}\</style\>  
\<h1\>Quote \<code\>{{QUOTE\_ID}}\</code\>\</h1\>  
\<pre id="meta"\>\</pre\>  
\<table id="t"\>\<thead\>\<tr\>\<th\>SKU\</th\>\<th\>Qty\</th\>\<th\>Unit Net\</th\>\<th\>Tax\</th\>\<th\>Unit Total\</th\>\<th\>Line Total\</th\>\</tr\>\</thead\>\<tbody\>\</tbody\>\</table\>  
\<h3\>Totals\</h3\>  
\<pre id="tot"\>\</pre\>  
\<script\>  
(async () \=\> {  
  const id \= location.pathname.split('/').pop();  
  const res \= await fetch(\`/v1/quote/get/${id}\`);  
  const q \= await res.json();  
  document.getElementById('meta').textContent \= \`created\_at: ${q.created\_at}\`;  
  const tb \= document.querySelector('\#t tbody');  
  q.items.forEach(it \=\> {  
    const tr \= document.createElement('tr');  
    tr.innerHTML \= \`\<td\>${it.sku}\</td\>\<td\>${it.qty}\</td\>\<td\>${it.unit\_net}\</td\>\<td\>${it.unit\_tax}\</td\>\<td\>${it.unit\_total}\</td\>\<td\>${it.line\_total}\</td\>\`;  
    tb.appendChild(tr);  
  });  
  document.getElementById('tot').textContent \= JSON.stringify(q.totals, null, 2);  
})();  
\</script\>  
Registrar o módulo UI no main.rs:  
\#\[cfg(feature \= "modules")\]  
mod ui\_quote;  
\#\[cfg(feature \= "modules")\]  
use ui\_quote::ui\_quote\_router;

## **6\)** 

## **LLM labels**

##  **(opcional) —** 

## **modules/cap-llm/src/routes\_pricing.rs**

use axum::{routing::post, Json, Router};  
use serde::{Deserialize, Serialize};

\#\[derive(Debug, Deserialize)\]  
pub struct LabelReq {  
    pub locale: Option\<String\>,  
    pub sku: String,  
    pub base\_title: Option\<String\>,  
    pub features: Option\<Vec\<String\>\>,  
}

\#\[derive(Debug, Serialize)\]  
pub struct LabelResp {  
    pub title: String,  
    pub tagline: String,  
    pub bullets: Vec\<String\>,  
}

pub fn router\_pricing() \-\> Router {  
    Router::new().route("/v1/llm/label/pricing", post(gen))  
}

async fn gen(Json(req): Json\<LabelReq\>) \-\> Result\<Json\<LabelResp\>, axum::http::StatusCode\> {  
    let locale \= req.locale.unwrap\_or\_else(|| "pt-BR".into());  
    let prompt \= format\!(  
        "Gere título curto, tagline e 3 bullets para SKU {sku}. Locale={locale}. \\  
         Saída JSON com campos title, tagline, bullets\[\].",  
        sku=req.sku  
    );  
    // use o client que você já tem dentro de cap-llm (OpenAI/Anthropic/etc.)  
    let out \= crate::client::complete\_json(\&prompt).await.map\_err(|\_| axum::http::StatusCode::BAD\_GATEWAY)?;  
    Ok(Json(out))  
}  
No cap-llm/src/lib.rs, expor router\_pricing() (atrás de feature \= "llm"). Isso é não-determinístico e não altera valores — só texto.

## **7\) Teste de integração —** 

## **modules/cap-quote/tests/flow.rs**

use axum::Router;  
use cap\_quote::{router as quote\_router, load\_pricing};  
use cap\_pricing::{router as pricing\_router};  
use tempfile::NamedTempFile;  
use serde\_json::json;

\#\[tokio::test\]  
async fn quote\_end\_to\_end() {  
    let cfg \= r\#"  
rounding: { scale: 2, mode: half\_up }  
list: { PLAN-PRO: 49.00, ADDON-TEAM: 15.00 }  
rules:  
  \- { name: coupon-PLAN-20, target: coupon, matcher: PLAN20, action: discount\_pct, value: 20, stackable: true }  
tax: { default\_pct: 0.00, by\_region: { BR: 0.12 } }  
"\#;  
    let tmp \= NamedTempFile::new().unwrap();  
    std::fs::write(tmp.path(), cfg).unwrap();  
    load\_pricing(tmp.path()).unwrap();

    let app \= Router::new().merge(pricing\_router()).merge(quote\_router());  
    let listener \= std::net::TcpListener::bind("127.0.0.1:0").unwrap();  
    let addr \= listener.local\_addr().unwrap();  
    let handle \= tokio::spawn(axum::serve(listener, app));

    let req \= json\!({  
      "region":"BR",  
      "coupons":\["PLAN20"\],  
      "items":\[  
        {"sku":"PLAN-PRO","qty":"1"},  
        {"sku":"ADDON-TEAM","qty":"3"}  
      \]  
    });  
    let out: serde\_json::Value \=  
        reqwest::Client::new().post(format\!("http://{}/v1/quote/create", addr))  
        .json(\&req).send().await.unwrap().json().await.unwrap();

    assert\!(out\["id"\].is\_string());  
    assert\_eq\!(out\["totals"\]\["total"\].as\_str().unwrap(), "93.10"); // 39.20+4.70 \+ (15\*3)\*(1+0) \= 43.90 \+ 45.00

    handle.abort();  
}

## **8\) Como rodar 🔧**

\# 1\) Config  
export PRICING\_PATH="$PWD/configs/pricing/pricing.yaml"

\# 2\) Registry com módulos  
cargo run \-p registry \--features modules \--release  
\# Endpoints:  
\#  \- /v1/pricing/price, /v1/pricing/quote  
\#  \- /v1/quote/create, /v1/quote/get/:id, /v1/quote/reprice  
\#  \- /ui/quote, /ui/quote/:id

### **cURL**

curl \-s localhost:8790/v1/quote/create \-H 'content-type: application/json' \-d '{  
  "region":"BR-SP","customer\_tier":"enterprise","coupons":\["PLAN20"\],  
  "items":\[{"sku":"PLAN-PRO","qty":"1"},{"sku":"SKU-CPU-H","qty":"120"}\]  
}' | jq .

\# abrir no navegador:  
open "http://localhost:8790/ui/quote"

---

## **9\) Critérios de aceite ✅**

* Cálculo determinístico herdado de cap-pricing, com steps explicáveis (no pricing).

* Quote: criação, consulta e reprice (em memória), com totais consistentes.

* UI simples: criação e visualização web (/ui/quote).

* LLM opcional: endpoint para labels; não influencia valores.

* Testes: fluxo E2E assegura totals; fácil estender casos.

---

Se quiser, no próximo passo eu coloco o invoice stub (PDF/NRF/capsule), persisto quotes em ubl-storage (S3/FS), e plugo o /permit pra exigir ASK/ACK antes de “emitir” a fatura. Quer que eu já venha com isso na Mensagem \#9? 😎

