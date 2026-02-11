Booora\! **Mensagem \#11: Verificação da cápsula \+ Pre-signed S3 \+ Webhook `invoice-issued` (com retry/backoff)** ✅📦🔏📣

Meta: depois de gravar cápsula e hop (msg \#10), agora:

1. validar **cápsula \+ cadeia de receipts** (CLI e API),  
2. expor **download seguro** via **S3 pre-signed**,  
3. emitir **webhook** idempotente com retry/backoff, para integrar produto/ERP.

---

# **PATCH**

## **1\) `ubl` CLI — `verify` (capsule \+ receipts)**

### **`tools/ubl-cli/src/main.rs` (novo subcomando)**

use anyhow::{Result, bail};  
use clap::{Parser, Subcommand};  
use std::fs;  
use ubl\_capsule::{Capsule, Receipt};

\#\[derive(Parser)\]  
\#\[command(name="ubl")\]  
struct Cli {  
    \#\[command(subcommand)\]  
    cmd: Cmd  
}

\#\[derive(Subcommand)\]  
enum Cmd {  
    /// Verifica cápsula e cadeia de receipts (arquivos .nrf)  
    Verify {  
        /// caminho do arquivo da cápsula .nrf  
        \#\[arg(long)\]  
        cap: String,  
        /// diretório com receipts da cápsula (receipts/\<cap\_id\>/\*.nrf)  
        \#\[arg(long)\]  
        receipts\_dir: String,  
    }  
}

fn main() \-\> Result\<()\> {  
    let cli \= Cli::parse();  
    match cli.cmd {  
        Cmd::Verify { cap, receipts\_dir } \=\> verify\_cmd(\&cap, \&receipts\_dir),  
    }  
}

fn verify\_cmd(cap\_path: \&str, rc\_dir: \&str) \-\> Result\<()\> {  
    let cap\_bytes \= fs::read(cap\_path)?;  
    let cap \= Capsule::from\_nrf\_bytes(\&cap\_bytes)?;  
    cap.verify\_seal()?; // checa assinatura e id (domain/scope/aud)

    // carrega receipts ordenando por ts (ou pelo id/file name)  
    let mut paths \= fs::read\_dir(rc\_dir)?  
        .filter\_map(|e| e.ok())  
        .map(|e| e.path())  
        .filter(|p| p.extension().map(|x| x=="nrf").unwrap\_or(false))  
        .collect::\<Vec\<\_\>\>();  
    paths.sort();

    let of \= cap.id\_bytes();  
    let mut prev: Option\<\[u8;32\]\> \= None;  
    for p in paths {  
        let b \= fs::read(\&p)?;  
        let r \= Receipt::from\_nrf\_bytes(\&b)?;  
        r.verify\_of(of)?;  
        r.verify\_chain\_prev(prev)?;  
        r.verify\_sig()?; // domain separation "ubl-receipt/1.0"  
        prev \= Some(r.id\_bytes());  
    }

    println\!("OK: capsule+receipts válidos ({} receipts)", paths.len());  
    Ok(())  
}

Resultado: `ubl verify --cap capsules/invoice/<iid>/<capid>.nrf --receipts_dir receipts/<capid>`  
Saída: `OK: capsule+receipts válidos (N receipts)`

---

## **2\) `ubl-storage` — S3 pre-signed URL (opcional)**

### **`crates/ubl-storage/src/lib.rs`**

\#\[async\_trait::async\_trait\]  
pub trait Storage: Send \+ Sync {  
    async fn put(\&self, key: String, bytes: Vec\<u8\>) \-\> anyhow::Result\<()\>;  
    async fn get(\&self, key: \&str) \-\> anyhow::Result\<Option\<Vec\<u8\>\>\>;  
    async fn list\_prefix(\&self, prefix: \&str) \-\> anyhow::Result\<Vec\<String\>\>;  
    /// URL temporária para download; \`None\` se backend não suporta  
    async fn presign\_get(\&self, key: \&str, seconds: u64) \-\> anyhow::Result\<Option\<String\>\>;  
}

### **`crates/ubl-storage/src/fs.rs` (FS: não suporta → None)**

\#\[async\_trait::async\_trait\]  
impl Storage for FsStore {  
    async fn presign\_get(\&self, \_key: \&str, \_seconds: u64) \-\> anyhow::Result\<Option\<String\>\> {  
        Ok(None)  
    }  
}

### **`crates/ubl-storage/src/s3.rs` (S3: suporta)**

use aws\_sdk\_s3::presigning::PresigningConfig;  
\#\[async\_trait::async\_trait\]  
impl Storage for S3Store {  
    async fn presign\_get(\&self, key: \&str, seconds: u64) \-\> Result\<Option\<String\>\> {  
        let req \= self.client.get\_object()  
            .bucket(\&self.bucket)  
            .key(self.key(key));  
        let pc \= PresigningConfig::expires\_in(std::time::Duration::from\_secs(seconds))?;  
        let presigned \= req.presigned(pc).await?;  
        Ok(Some(presigned.uri().to\_string()))  
    }  
}

---

## **3\) `services/registry` — API: verify \+ download \+ webhook**

### **3.1 Endpoints**

* `GET /v1/capsules/:invoice_id/latest` → info \+ **pre-signed URL** (se S3) ou path relativo (FS).  
* `POST /v1/capsules/:invoice_id/verify` → valida cápsula \+ receipts; `200 OK {valid:true, receipts:N}`.  
* **Webhook**: ao “approve/reject”, POST assíncrono para `$WEBHOOK_INVOICE_ISSUED`.

### **3.2 Router**

// services/registry/src/http.rs (ou onde define rotas)  
use axum::{routing::{get, post}, Router};  
pub fn router(ctx: Ctx) \-\> Router {  
    Router::new()  
        .route("/v1/capsules/:invoice\_id/latest", get(capsule\_latest))  
        .route("/v1/capsules/:invoice\_id/verify", post(capsule\_verify))  
        // ...já existentes...  
        .with\_state(ctx)  
}

### **3.3 Handlers**

use axum::{extract::{Path, State}, Json};  
use serde::Serialize;  
use tokio::fs;  
use std::path::Path as FsPath;

\#\[derive(Serialize)\]  
struct LatestResp {  
    invoice\_id: String,  
    cap\_id: String,  
    size: u64,  
    presigned\_url: Option\<String\>,  
    fs\_path: Option\<String\>,  
}

pub async fn capsule\_latest(  
    State(ctx): State\<Ctx\>,  
    Path(invoice\_id): Path\<String\>,  
) \-\> Result\<Json\<LatestResp\>, StatusCode\> {  
    let prefix \= format\!("capsules/invoice/{}/", invoice\_id);  
    let keys \= ctx.store.list\_prefix(\&prefix).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
    let Some(latest) \= keys.iter().filter(|k| k.ends\_with(".nrf")).max().cloned() else {  
        return Err(StatusCode::NOT\_FOUND);  
    };

    let size \= if let Some(root) \= ctx.store.fs\_root() { // helper opcional no FS  
        let p \= FsPath::new(\&root).join(\&latest);  
        fs::metadata(p).await.ok().map(|m| m.len()).unwrap\_or(0)  
    } else { 0 };

    let presigned \= ctx.store.presign\_get(\&latest, 300).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
    let fs\_path \= if presigned.is\_none() { Some(latest.clone()) } else { None };

    let cap\_id \= latest.rsplit('/').next().unwrap\_or\_default().trim\_end\_matches(".nrf").to\_string();  
    Ok(Json(LatestResp{  
        invoice\_id, cap\_id, size, presigned\_url: presigned, fs\_path  
    }))  
}

\#\[derive(serde::Deserialize)\]  
struct VerifyReq { \#\[serde(default)\] cap\_id: Option\<String\> }

\#\[derive(serde::Serialize)\]  
struct VerifyResp { valid: bool, receipts: usize }

pub async fn capsule\_verify(  
    State(ctx): State\<Ctx\>,  
    Path(invoice\_id): Path\<String\>,  
    Json(req): Json\<VerifyReq\>,  
) \-\> Result\<Json\<VerifyResp\>, StatusCode\> {  
    // resolve cap key  
    let prefix \= format\!("capsules/invoice/{}/", invoice\_id);  
    let keys \= ctx.store.list\_prefix(\&prefix).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
    let cap\_key \= if let Some(cid) \= req.cap\_id {  
        format\!("{prefix}{cid}.nrf")  
    } else {  
        keys.iter().filter(|k| k.ends\_with(".nrf")).max().cloned().ok\_or(StatusCode::NOT\_FOUND)?  
    };

    let cap\_bytes \= ctx.store.get(\&cap\_key).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?  
        .ok\_or(StatusCode::NOT\_FOUND)?;  
    let cap \= ubl\_capsule::Capsule::from\_nrf\_bytes(\&cap\_bytes).map\_err(|\_| StatusCode::BAD\_REQUEST)?;  
    cap.verify\_seal().map\_err(|\_| StatusCode::BAD\_REQUEST)?;

    let rc\_prefix \= format\!("receipts/{}/", hex::encode(cap.id\_bytes()));  
    let mut rc \= ctx.store.list\_prefix(\&rc\_prefix).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
    rc.retain(|k| k.ends\_with(".nrf"));  
    rc.sort();

    let mut prev \= None;  
    for k in \&rc {  
        let b \= ctx.store.get(k).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?  
            .ok\_or(StatusCode::NOT\_FOUND)?;  
        let r \= ubl\_capsule::Receipt::from\_nrf\_bytes(\&b).map\_err(|\_| StatusCode::BAD\_REQUEST)?;  
        r.verify\_of(cap.id\_bytes()).map\_err(|\_| StatusCode::BAD\_REQUEST)?;  
        r.verify\_chain\_prev(prev).map\_err(|\_| StatusCode::BAD\_REQUEST)?;  
        r.verify\_sig().map\_err(|\_| StatusCode::BAD\_REQUEST)?;  
        prev \= Some(r.id\_bytes());  
    }  
    Ok(Json(VerifyResp{ valid: true, receipts: rc.len() }))  
}

### **3.4 Webhook (emitir ao approve/reject)**

No mesmo handler do APPROVE/REJECT que criamos na \#10, **depois** de gravar cápsula \+ hop, dispare:

\#\[derive(serde::Serialize)\]  
struct InvoiceIssued {  
    invoice\_id: String,  
    cap\_id: String,  
    url: Option\<String\>,   // presigned (S3) se houver  
    path: Option\<String\>,  // FS path se sem S3  
    verdict: String,       // "ACK" | "NACK"  
    ts: i64,  
}

async fn enqueue\_webhook(ctx: \&Ctx, invoice\_id: \&str, cap\_id: \&str, verdict: \&str) {  
    if let Ok(Some(url)) \= ctx.store.presign\_get(\&format\!("capsules/invoice/{invoice\_id}/{cap\_id}.nrf"), 300).await {  
        let evt \= InvoiceIssued{  
            invoice\_id: invoice\_id.to\_string(),  
            cap\_id: cap\_id.to\_string(),  
            url: Some(url),  
            path: None,  
            verdict: verdict.to\_string(),  
            ts: time::OffsetDateTime::now\_utc().unix\_timestamp\_nanos(),  
        };  
        tokio::spawn(webhook\_retry(evt));  
    } else {  
        let evt \= InvoiceIssued{  
            invoice\_id: invoice\_id.to\_string(),  
            cap\_id: cap\_id.to\_string(),  
            url: None,  
            path: Some(format\!("capsules/invoice/{invoice\_id}/{cap\_id}.nrf")),  
            verdict: verdict.to\_string(),  
            ts: time::OffsetDateTime::now\_utc().unix\_timestamp\_nanos(),  
        };  
        tokio::spawn(webhook\_retry(evt));  
    }  
}

async fn webhook\_retry(evt: InvoiceIssued) {  
    let Some(url) \= std::env::var("WEBHOOK\_INVOICE\_ISSUED").ok() else { return; };  
    let mut backoff \= \[2u64, 5, 15, 30, 60, 120\]; // segundos  
    let client \= reqwest::Client::new();  
    let key \= format\!("invoice:{}:{}", evt.invoice\_id, evt.cap\_id);  
    for (i, delay) in backoff.iter().enumerate() {  
        let res \= client.post(\&url)  
            .header("content-type", "application/json")  
            .header("idempotency-key", \&key)  
            .json(\&evt)  
            .send().await;  
        if matches\!(res, Ok(r) if r.status().is\_success()) { return; }  
        tokio::time::sleep(std::time::Duration::from\_secs(\*delay)).await;  
        if i \== backoff.len()-1 {  
            tracing::warn\!("webhook failed after retries: {:?}", res.err());  
        }  
    }  
}

**Idempotência:** header `idempotency-key` \= `invoice_id/cap_id`.  
**Backoff:** 2,5,15,30,60,120s (ajustável via env, se quiser).

---

## **4\) CI: provas E2E**

No job `registry-integration` (ci.yml), depois do fluxo de approve da \#10, acrescente:

\# 1\) /v1/capsules/:iid/latest  
iid="${iid:?missing}"  
latest="$(curl \-s "http://127.0.0.1:$PORT/v1/capsules/$iid/latest")"  
echo "$latest" | jq .  
cap\_id="$(echo "$latest" | jq \-r .cap\_id)"  
test \-n "$cap\_id"

\# 2\) /v1/capsules/:iid/verify  
curl \-s \-X POST "http://127.0.0.1:$PORT/v1/capsules/$iid/verify" \-H 'content-type: application/json' \\  
  \-d '{}' | tee /tmp/verify.json  
jq \-e '.valid==true' /tmp/verify.json \>/dev/null

\# 3\) ubl verify (CLI) — usando FS paths se presigned ausente  
if path=$(echo "$latest" | jq \-r .fs\_path) && \[ "$path" \!= "null" \]; then  
  cap\_path="$RUNNER\_TEMP/store/$path"  
  receipts\_dir="$RUNNER\_TEMP/store/receipts/$(basename "$cap\_path" .nrf)"  
  ubl verify \--cap "$cap\_path" \--receipts\_dir "$receipts\_dir"  
fi

Em S3, o teste usa só o endpoint `/verify` (sem baixar o `.nrf` via presigned na CI pública).

---

## **5\) Operação (ENV) 🧩**

* **S3**: `INVOICE_S3_BUCKET`, `INVOICE_S3_PREFIX`, `AWS_*` (region/creds)  
* **Webhook**: `WEBHOOK_INVOICE_ISSUED=https://…/your-hook`  
* **Chaves**:  
  * `CAPSULE_KID`, `CAPSULE_SK_HEX` (64 hex) — **seal**  
  * `RECEIPT_SK_HEX` (64 hex) — **hops**  
* **Ports/URLs**: `PORT`, `CDN_BASE` (já setados antes)

---

## **6\) Aceite ✅**

* `GET /v1/capsules/:invoice_id/latest` → **cap\_id** \+ **presigned\_url** (S3) ou **fs\_path** (FS).  
* `POST /v1/capsules/:invoice_id/verify` → `{"valid":true,"receipts":N}`.  
* `ubl verify --cap <file> --receipts_dir <dir>` → imprime `OK`.  
* Ao `approve/reject`:  
  * emite **webhook idempotente** com retry/backoff,  
  * inclui link temporário (S3) ou path (FS).  
* CI cobre o **E2E** acima.

---

Quer que eu já **commite** esse bloco (CLI \+ storage \+ API \+ CI) como PR “feat(verify): capsule verify \+ presign \+ webhook invoice-issued”? Posso deixar **atrás de feature** (`s3`) e com **WEBHOOK** desligado por padrão (só liga via env). 🚀

