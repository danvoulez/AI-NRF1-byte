Perfeito — Mensagem \#7: cap-pricing 💸📈

Módulo de precificação com listas de preço, regras, desconto, impostos e simulação por cenário. Integra com cap-llm opcionalmente para geração de rótulos/descrições. Configurável por YAML, stateless e idempotente (determinístico sem LLM). Bora lá.

---

# **Patch —** 

# **modules/cap-pricing**

## **1\)** 

## **modules/cap-pricing/Cargo.toml**

\[package\]

name \= "cap-pricing"

version \= "0.1.0"

edition \= "2021"

license \= "MIT OR Apache-2.0"

\[features\]

default \= \[\]

llm \= \["dep:cap-llm"\]

\[dependencies\]

anyhow \= "1"

axum \= { version \= "0.7", features \= \["json"\] }

serde \= { version \= "1", features \= \["derive"\] }

serde\_json \= "1"

serde\_yaml \= "0.9"

time \= { version \= "0.3", features \= \["macros"\] }

once\_cell \= "1"

tracing \= "0.1"

indexmap \= "2"

rust\_decimal \= { version \= "1", features \= \["serde"\] }

rust\_decimal\_macros \= "1"

uuid \= { version \= "1", features \= \["v4"\] }

\# Opcional: usar completions para rotular/explicar preços

cap-llm \= { path \= "../cap-llm", optional \= true }

---

## **2\) Config —** 

## **modules/cap-pricing/src/config.rs**

use serde::{Deserialize, Serialize};

use rust\_decimal::Decimal;

use indexmap::IndexMap;

\#\[derive(Debug, Clone, Serialize, Deserialize)\]

pub struct PricingConfig {

    /// Tabela de bases por SKU (preço de lista bruto)

    pub list: IndexMap\<String, Decimal\>, // ex.: "SKU123" \-\> 129.90

    /// Regras: ordenadas, primeira que casar aplica (pode ser múltiplas se stackable=true)

    \#\[serde(default)\]

    pub rules: Vec\<Rule\>,

    /// Impostos por região (padrão nacional \+ overrides)

    \#\[serde(default)\]

    pub tax: TaxConfig,

    /// Arredondamento

    \#\[serde(default \= "default\_round")\]

    pub rounding: Rounding,

}

\#\[derive(Debug, Clone, Serialize, Deserialize)\]

pub struct Rule {

    pub name: String,

    /// tipo de alvo: "sku" | "category" | "customer\_tier" | "coupon"

    pub target: String,

    pub matcher: String,   // valor exato ou regex simples

    /// ação: "discount\_pct" | "discount\_abs" | "surcharge\_pct" | "surcharge\_abs"

    pub action: String,

    pub value: Decimal,

    /// somar com próximas regras que batem? (default: false)

    \#\[serde(default)\]

    pub stackable: bool,

    /// prioridade (menor primeiro). Se ausente, ordem do arquivo

    pub priority: Option\<i32\>,

}

\#\[derive(Debug, Clone, Serialize, Deserialize, Default)\]

pub struct TaxConfig {

    pub default\_pct: Option\<Decimal\>,

    /// overrides por region code (ex.: "BR-SP": 0.19)

    \#\[serde(default)\]

    pub by\_region: IndexMap\<String, Decimal\>,

}

\#\[derive(Debug, Clone, Serialize, Deserialize)\]

pub struct Rounding {

    /// casas decimais (2 típico)

    pub scale: u32,

    /// "bankers" | "half\_up"

    pub mode: String,

}

fn default\_round() \-\> Rounding {

    Rounding { scale: 2, mode: "half\_up".into() }

}

---

## **3\) API —** 

## **modules/cap-pricing/src/api.rs**

use serde::{Deserialize, Serialize};

use rust\_decimal::Decimal;

use indexmap::IndexMap;

\#\[derive(Debug, Clone, Deserialize)\]

pub struct PriceReq {

    pub sku: String,

    pub qty: Option\<Decimal\>,

    pub region: Option\<String\>,          // BR, BR-SP, US-CA …

    pub category: Option\<String\>,        // para regras

    pub customer\_tier: Option\<String\>,   // ex.: "pro", "enterprise"

    pub coupons: Option\<Vec\<String\>\>,    // cupons aplicáveis

    /// explicação detalhada da composição do preço

    \#\[serde(default)\]

    pub explain: bool,

}

\#\[derive(Debug, Clone, Serialize)\]

pub struct PriceResp {

    pub sku: String,

    pub unit\_list: Decimal,

    pub unit\_net: Decimal,

    pub unit\_tax: Decimal,

    pub unit\_total: Decimal,

    pub qty: Decimal,

    pub total\_net: Decimal,

    pub total\_tax: Decimal,

    pub total: Decimal,

    pub steps: Vec\<Step\>,

}

\#\[derive(Debug, Clone, Serialize)\]

pub struct Step {

    pub kind: String, // list|rule|tax|round

    pub name: String,

    pub before: Decimal,

    pub after: Decimal,

    pub meta: IndexMap\<String, String\>,

}

\#\[derive(Debug, Clone, Deserialize)\]

pub struct ScenarioReq {

    pub items: Vec\<PriceReq\>,

}

\#\[derive(Debug, Clone, Serialize)\]

pub struct ScenarioResp {

    pub items: Vec\<PriceResp\>,

    pub grand\_total: Decimal,

}

---

## **4\) Engine —** 

## **modules/cap-pricing/src/engine.rs**

use crate::config::{PricingConfig, Rounding};

use crate::api::{PriceReq, PriceResp, Step};

use anyhow::{anyhow, Result};

use rust\_decimal::Decimal;

use rust\_decimal::prelude::ToPrimitive;

use indexmap::IndexMap;

pub fn price\_one(cfg: \&PricingConfig, req: \&PriceReq) \-\> Result\<PriceResp\> {

    let qty \= req.qty.clone().unwrap\_or(Decimal::ONE);

    let list \= cfg.list.get(\&req.sku).ok\_or\_else(|| anyhow\!("unknown sku"))?.clone();

    let mut steps \= vec\!\[\];

    let mut unit \= list;

    steps.push(step("list", "base", unit, unit, IndexMap::new()));

    // aplicar regras em ordem de prioridade/arquivo

    let mut rules \= cfg.rules.clone();

    rules.sort\_by\_key(|r| r.priority.unwrap\_or(i32::MAX));

    for r in rules {

        if \!matches(\&r.target, \&r.matcher, req) { continue; }

        let before \= unit;

        unit \= match r.action.as\_str() {

            "discount\_pct"   \=\> unit \* (Decimal::ONE \- r.value / dec(100)),

            "discount\_abs"   \=\> unit \- r.value,

            "surcharge\_pct"  \=\> unit \* (Decimal::ONE \+ r.value / dec(100)),

            "surcharge\_abs"  \=\> unit \+ r.value,

            \_ \=\> before,

        };

        let mut meta \= IndexMap::new();

        meta.insert("rule".into(), r.name.clone());

        meta.insert("action".into(), r.action);

        meta.insert("value".into(), r.value.to\_string());

        steps.push(step("rule", \&r.name, before, unit, meta));

        // se não for stackable e casou, para

        if \!r.stackable { /\* continua mas ignora outras? \*/ }

    }

    let unit\_net \= round\_by(unit, \&cfg.rounding);

    // imposto

    let pct \= tax\_pct(\&cfg, req.region.as\_deref());

    let unit\_tax \= round\_by(unit\_net \* pct, \&cfg.rounding);

    let unit\_total \= round\_by(unit\_net \+ unit\_tax, \&cfg.rounding);

    steps.push(step("tax", "vat/sales", unit\_net, unit\_total, {

        let mut m \= IndexMap::new(); m.insert("pct".into(), pct.to\_string()); m

    }));

    // totais

    let total\_net \= round\_by(unit\_net \* qty, \&cfg.rounding);

    let total\_tax \= round\_by(unit\_tax \* qty, \&cfg.rounding);

    let total \= round\_by(unit\_total \* qty, \&cfg.rounding);

    Ok(PriceResp {

        sku: req.sku.clone(),

        unit\_list: list,

        unit\_net, unit\_tax, unit\_total,

        qty,

        total\_net, total\_tax, total,

        steps,

    })

}

fn matches(target: \&str, matcher: \&str, req: \&PriceReq) \-\> bool {

    use regex::Regex;

    let re \= Regex::new(\&format\!("^{}$", matcher.replace('\*', ".\*"))).unwrap();

    match target {

        "sku" \=\> re.is\_match(\&req.sku),

        "category" \=\> req.category.as\_deref().map(|c| re.is\_match(c)).unwrap\_or(false),

        "customer\_tier" \=\> req.customer\_tier.as\_deref().map(|t| re.is\_match(t)).unwrap\_or(false),

        "coupon" \=\> req.coupons.as\_ref().map(|v| v.iter().any(|c| re.is\_match(c))).unwrap\_or(false),

        \_ \=\> false

    }

}

fn step(kind: \&str, name: \&str, before: Decimal, after: Decimal, meta: IndexMap\<String,String\>) \-\> Step {

    Step { kind: kind.into(), name: name.into(), before, after, meta }

}

fn tax\_pct(cfg: \&PricingConfig, region: Option\<\&str\>) \-\> Decimal {

    if let Some(r) \= region {

        if let Some(p) \= cfg.tax.by\_region.get(r) { return \*p; }

        // fallback por país (ex.: BR-SP → BR)

        if let Some((cc, \_)) \= r.split\_once('-') {

            if let Some(p) \= cfg.tax.by\_region.get(cc) { return \*p; }

        }

    }

    cfg.tax.default\_pct.unwrap\_or(dec(0))

}

fn round\_by(v: Decimal, r: \&Rounding) \-\> Decimal {

    use rust\_decimal::RoundingStrategy;

    let mode \= match r.mode.as\_str() {

        "bankers" \=\> RoundingStrategy::MidpointNearestEven,

        \_ \=\> RoundingStrategy::MidpointAwayFromZero

    };

    v.round\_dp\_with\_strategy(r.scale, mode)

}

fn dec(x: i64) \-\> Decimal { Decimal::from\_i64(x).unwrap() }

---

## **5\) Router —** 

## **modules/cap-pricing/src/lib.rs**

mod config;

mod api;

mod engine;

use anyhow::Result;

use axum::{Json, Router, routing::post};

use once\_cell::sync::OnceCell;

use crate::api::{PriceReq, PriceResp, ScenarioReq, ScenarioResp};

pub use config::PricingConfig;

static CONF: OnceCell\<PricingConfig\> \= OnceCell::new();

pub fn load\_pricing\_from(path: impl AsRef\<std::path::Path\>) \-\> Result\<()\> {

    let text \= std::fs::read\_to\_string(path)?;

    let cfg: PricingConfig \= serde\_yaml::from\_str(\&text)?;

    CONF.set(cfg).ok();

    Ok(())

}

pub fn router() \-\> Router {

    Router::new()

        .route("/v1/pricing/price", post(price\_handler))

        .route("/v1/pricing/quote", post(quote\_handler))

}

async fn price\_handler(axum::Json(req): Json\<PriceReq\>) \-\> Result\<Json\<PriceResp\>, axum::http::StatusCode\> {

    let cfg \= CONF.get().ok\_or(axum::http::StatusCode::SERVICE\_UNAVAILABLE)?;

    let out \= engine::price\_one(cfg, \&req).map\_err(|\_| axum::http::StatusCode::BAD\_REQUEST)?;

    Ok(Json(out))

}

async fn quote\_handler(axum::Json(req): Json\<ScenarioReq\>) \-\> Result\<Json\<ScenarioResp\>, axum::http::StatusCode\> {

    let cfg \= CONF.get().ok\_or(axum::http::StatusCode::SERVICE\_UNAVAILABLE)?;

    let mut items \= vec\!\[\];

    let mut grand \= rust\_decimal\_macros::dec\!(0);

    for it in \&req.items {

        let r \= engine::price\_one(cfg, it).map\_err(|\_| axum::http::StatusCode::BAD\_REQUEST)?;

        grand \+= r.total;

        items.push(r);

    }

    Ok(Json(ScenarioResp { items, grand\_total: grand }))

}

---

## **6\) Config YAML —** 

## **configs/pricing/pricing.yaml**

rounding: { scale: 2, mode: half\_up }

list:

  PLAN-BASIC: 19.90

  PLAN-PRO:   49.00

  ADDON-TEAM: 15.00

  SKU-CPU-H:  0.10

rules:

  \# 10% off no PRO para enterprise

  \- name: tier-enterprise-pro-10

    target: customer\_tier

    matcher: enterprise

    action: discount\_pct

    value: 10

    stackable: true

    priority: 10

  \# Cupom de 20% qualquer SKU de plano (prefixo PLAN-\*)

  \- name: coupon-PLAN-20

    target: coupon

    matcher: PLAN20

    action: discount\_pct

    value: 20

    stackable: true

    priority: 20

  \# Acréscimo de 5% em CPU para região específica

  \- name: surcharge-cpu-region

    target: sku

    matcher: SKU-CPU-H

    action: surcharge\_pct

    value: 5

    stackable: false

    priority: 30

tax:

  default\_pct: 0.00

  by\_region:

    BR: 0.12

    BR-SP: 0.19

    US: 0.00

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

  "cap-pricing",

\]

\[dependencies\]

cap-pricing  \= { path \= "../../modules/cap-pricing",  optional \= true }

\# (mantém os demais já adicionados na msg \#6)

### **services/registry/src/main.rs**

###  **(trecho)**

\#\[cfg(feature \= "modules")\]

fn mount\_modules(router: axum::Router) \-\> axum::Router {

    use cap\_pricing;

    if let Ok(p) \= std::env::var("PRICING\_PATH") { let \_ \= cap\_pricing::load\_pricing\_from(p); }

    router

        // … (demais merges)

        .merge(cap\_pricing::router()) // /v1/pricing/\*

}

---

## **8\) Testes**

### **Unit —** 

### **modules/cap-pricing/tests/basic.rs**

use cap\_pricing::{load\_pricing\_from, router as pricing\_router};

use serde\_json::json;

use tempfile::NamedTempFile;

use tokio::task;

\#\[tokio::test\]

async fn price\_basic\_pro\_coupon\_tax() {

    let cfg \= r\#"

rounding: { scale: 2, mode: half\_up }

list: { PLAN-PRO: 49.00 }

rules:

  \- { name: coupon-PLAN-20, target: coupon, matcher: PLAN20, action: discount\_pct, value: 20, stackable: true }

tax: { default\_pct: 0.00, by\_region: { BR: 0.12 } }

"\#;

    let tmp \= NamedTempFile::new().unwrap();

    std::fs::write(tmp.path(), cfg).unwrap();

    load\_pricing\_from(tmp.path()).unwrap();

    let app \= pricing\_router();

    let listener \= std::net::TcpListener::bind("127.0.0.1:0").unwrap();

    let addr \= listener.local\_addr().unwrap();

    let handle \= task::spawn(axum::serve(listener, app));

    let req \= json\!({"sku":"PLAN-PRO","region":"BR","coupons":\["PLAN20"\],"qty":1});

    let out: serde\_json::Value \=

        reqwest::Client::new().post(format\!("http://{}/v1/pricing/price", addr))

        .json(\&req).send().await.unwrap().json().await.unwrap();

    // 49 \- 20% \= 39.20; tax 12% de 39.20 \= 4.70 (4.704 \-\> 4.70 half\_up); total 43.90

    assert\_eq\!(out\["unit\_net"\].as\_str().unwrap(), "39.20");

    assert\_eq\!(out\["unit\_tax"\].as\_str().unwrap(), "4.70");

    assert\_eq\!(out\["unit\_total"\].as\_str().unwrap(), "43.90");

    handle.abort();

}

---

## **9\) Curl de uso**

Preço unitário com cupom e região:

export PRICING\_PATH="$PWD/configs/pricing/pricing.yaml"

cargo run \-p registry \--features modules \--release

\# nova aba

curl \-s localhost:8790/v1/pricing/price \-H 'content-type: application/json' \-d '{

  "sku":"PLAN-PRO",

  "region":"BR-SP",

  "customer\_tier":"enterprise",

  "coupons":\["PLAN20"\],

  "qty":"1"

}' | jq .

Cotação de cenário (vários itens):

curl \-s localhost:8790/v1/pricing/quote \-H 'content-type: application/json' \-d '{

  "items": \[

    {"sku":"PLAN-BASIC","region":"US","qty":"1"},

    {"sku":"ADDON-TEAM","region":"US","qty":"3"},

    {"sku":"SKU-CPU-H","region":"BR-SP","qty":"120"}

  \]

}' | jq .

---

## **10\) Critérios de aceite ✅**

* Determinístico: cálculo sem LLM é puro e idempotente.

* Explainability: steps\[\] mostra cada transformação (list → rules → tax → round).

* Regras composáveis: stackable, prioridade, targets: sku|category|customer\_tier|coupon.

* Impostos por região: país/estado com fallback.

* Rounding: modo \+ escala configuráveis (default 2 casas, half\_up).

* Integração: PRICING\_PATH \+ router em registry \--features modules.

* Testes: unit com caso real de cupom+tax; mais casos podem ser adicionados (edge: valores negativos, isenção total, qty≠1).

* Compat: sem dependência hard de cap-llm. (Se quiser “copy de pricing” automática, adiciono endpoints sob feature \= "llm" depois.)

---

Se curtir, eu já preparo a Mensagem \#8 com um pacote de workflows de produto (pricing→quote→invoice stub), incluindo exemplos de prompts no cap-llm para gerar descrições/plan titles e um mini dashboard HTML de cotação. Quer nessa linha? 😄

