Bora fechar com chave de ouro\! Mensagem \#10: Capsule \+ Receipts encadeadas \+ Storage S3 (opcional) 🔐🧬🗄️

Objetivo: quando um invoice sai de PENDING\_ASK → APPROVED/REJECTED, emitimos cápsula UBL (ai-nrf1) e anexamos hop receipt no registry. Persistência padrão em FS; S3 opcional via ubl-storage.

---

# **PATCH**

## **1\)** 

## **cap-invoice**

##  **— emitir cápsula no APPROVE/REJECT**

### **modules/cap-invoice/Cargo.toml**

\[features\]  
default \= \["capsule"\]  
storage-s3 \= \["ubl-storage/s3"\]  
capsule \= \["ubl\_capsule", "signers"\]

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
signers \= { path \= "../../impl/rust/signers", optional \= true }

### **modules/cap-invoice/src/capsule.rs**

\#\[cfg(feature="capsule")\]  
use anyhow::{Result, bail};  
\#\[cfg(feature="capsule")\]  
use serde\_json::json;  
\#\[cfg(feature="capsule")\]  
use time::OffsetDateTime;  
\#\[cfg(feature="capsule")\]  
use ubl\_capsule::{Capsule, Seal, Bytes};  
\#\[cfg(feature="capsule")\]  
use signers::ed25519::{Ed25519Keypair, Ed25519Signer};

\#\[cfg(feature="capsule")\]  
pub struct CapSigner {  
    pub kid: String,       // DID\#key (ASCII)  
    pub kp: Ed25519Keypair // 32b secret  
}

\#\[cfg(feature="capsule")\]  
impl CapSigner {  
    pub fn from\_env() \-\> Result\<Self\> {  
        let kid \= std::env::var("CAPSULE\_KID")  
            .unwrap\_or\_else(|\_| "did:ubl:lab512\#k1".to\_string());  
        let sk\_hex \= std::env::var("CAPSULE\_SK\_HEX")  
            .map\_err(|\_| anyhow::anyhow\!("CAPSULE\_SK\_HEX ausente"))?;  
        let sk \= hex::decode(\&sk\_hex)?.try\_into().map\_err(|\_| anyhow::anyhow\!("sk len"))?;  
        let kp \= Ed25519Keypair::from\_secret(sk);  
        Ok(Self { kid, kp })  
    }

    pub fn sign\_capsule(\&self, env: serde\_json::Value) \-\> Result\<Capsule\> {  
        // hdr minimal \+ exp curto (5 min)  
        let now \= OffsetDateTime::now\_utc();  
        let exp \= now.unix\_timestamp\_nanos() \+ 5 \* 60 \* 1\_000\_000\_000i64;  
        let signer \= Ed25519Signer::from\_keypair(self.kp.clone());  
        let mut seal \= Seal::new\_ed25519(self.kid.clone(), Bytes::from(\[0u8;64\])); // placeholder  
        let mut cap \= Capsule::new(env, None, \&mut seal, exp);  
        cap.sign\_with(|msg| signer.sign(msg))?;  
        Ok(cap)  
    }  
}

\#\[cfg(feature="capsule")\]  
pub fn invoice\_env(verdict: \&str, inv\_json: \&serde\_json::Value) \-\> serde\_json::Value {  
    json\!({  
        "v":"ubl-json/0.1.1",  
        "t":"record",  
        "agent":{"id":"svc:registry","name":"registry"},  
        "intent":{"kind":"ATTEST","name":"invoice"},  
        "decision":{"verdict": verdict},  
        "ctx":{"invoice": inv\_json},  
        "evidence":{"cids":\[\], "urls":\[\]},  
        "meta":{"app":"ai-nrf1","tenant":"lab512","user":"system"}  
    })  
}

### **modules/cap-invoice/src/lib.rs**

###  **—** 

### **approve/reject**

###  **emite cápsula e grava**

// ... imports anteriores  
\#\[cfg(feature="capsule")\] use crate::capsule::{CapSigner, invoice\_env};  
use ubl\_storage::Storage;

async fn approve\_invoice(State(ctx): State\<Ctx\>, Json(req): Json\<ApproveReq\>)  
 \-\> Result\<Json\<InvoiceStub\>, StatusCode\> {  
    let key \= format\!("invoices/{}.json", req.id);  
    let Some(mut inv): Option\<InvoiceStub\> \= ctx.store.get\_json(\&key).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)? else {  
        return Err(StatusCode::NOT\_FOUND)  
    };  
    inv.status \= InvoiceStatus::APPROVED;  
    ctx.store.put\_json(\&key, \&inv).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;

    // — emitir cápsula (opcional via feature)  
    \#\[cfg(feature="capsule")\]  
    {  
        let signer \= CapSigner::from\_env().map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
        let env \= invoice\_env("ACK", \&serde\_json::to\_value(\&inv).unwrap());  
        let cap \= signer.sign\_capsule(env).map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
        let bytes \= cap.to\_nrf\_bytes().map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
        let ck \= format\!("capsules/invoice/{}/{}.nrf", inv.id, cap.id\_hex());  
        ctx.store.put(\&ck, bytes).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
    }

    Ok(Json(inv))  
}

async fn reject\_invoice(State(ctx): State\<Ctx\>, Json(req): Json\<RejectReq\>)  
 \-\> Result\<Json\<InvoiceStub\>, StatusCode\> {  
    // idêntico, mas verdict "NACK"  
    // ...  
    \#\[cfg(feature="capsule")\]  
    {  
        let signer \= CapSigner::from\_env().map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
        let env \= invoice\_env("NACK", \&serde\_json::to\_value(\&inv).unwrap());  
        let cap \= signer.sign\_capsule(env).map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
        let bytes \= cap.to\_nrf\_bytes().map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
        let ck \= format\!("capsules/invoice/{}/{}.nrf", inv.id, cap.id\_hex());  
        ctx.store.put(\&ck, bytes).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
    }  
    Ok(Json(inv))  
}  
Env vars (capsule):

* CAPSULE\_KID — ex: did:ubl:lab512\#k1

* CAPSULE\_SK\_HEX — 64 hex (Ed25519 secret)

---

## **2\)** 

## **services/registry**

##  **—** 

## **hop receipt**

##  **a cada approve/reject**

### **crates/receipt**

###  **(já existe) — auxiliar simples**

Se ainda não houver construtor, adicione:

// crates/receipt/src/lib.rs  
use ubl\_capsule::{Receipt, Bytes};  
use time::OffsetDateTime;

pub fn make\_hop(of: \[u8;32\], prev: Option\<\[u8;32\]\>, kind: \&str, node: \&str, signer: \&impl Fn(&\[u8\])-\>\[u8;64\]) \-\> anyhow::Result\<Receipt\> {  
    let ts \= OffsetDateTime::now\_utc().unix\_timestamp\_nanos();  
    let payload \= Receipt::payload("ubl-receipt/1.0", of, prev, kind, node, ts);  
    let sig \= signer(\&payload.hash()) ;  
    Ok(Receipt{  
        domain: "ubl-receipt/1.0".into(),  
        of: Bytes::from(of),  
        prev: prev.map(Bytes::from),  
        kind: kind.into(),  
        node: node.into(),  
        ts,  
        sig: Bytes::from(sig),  
    })  
}

### **services/registry/src/modules/mod.rs**

###  **— encadear receipt**

No handler (ou middleware) que processa /v1/invoice/approve|reject, após gravar a cápsula:

\#\[cfg(feature="modules")\]  
{  
    use ubl\_capsule::{Capsule, Id32};  
    use receipt::make\_hop;  
    use signers::ed25519::Ed25519Signer;

    if let Some(bytes) \= ctx.store.get\_latest\_capsule\_for\_invoice(inv.id).await.ok().flatten() {  
        let cap \= Capsule::from\_nrf\_bytes(\&bytes).map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
        let of: \[u8;32\] \= cap.id\_bytes();  
        let prev \= ctx.store.get\_last\_receipt\_id(of).await.ok().flatten();

        let signer \= Ed25519Signer::from\_env("RECEIPT\_SK\_HEX") // helper seu no signers  
            .map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
        let hop \= make\_hop(of, prev, "invoice/approve", "svc:registry", &|m| signer.sign(m))  
            .map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;

        let rk \= format\!("receipts/{}/{}.nrf", hex::encode(of), hop.id\_hex());  
        ctx.store.put(\&rk, hop.to\_nrf\_bytes().unwrap()).await.map\_err(|\_| StatusCode::INTERNAL\_SERVER\_ERROR)?;  
    }  
}  
Env vars (receipts):

* RECEIPT\_SK\_HEX — 64 hex Ed25519 para hops

Store helpers em ubl-storage (FS/S3):

* get\_latest\_capsule\_for\_invoice(uuid) → Option\<Vec\<u8\>\>

* get\_last\_receipt\_id(of) → Option\<\[u8;32\]\>

* put(key, bytes) / get(key)

Implementa no FsStore simples (pega maior timestamp no prefixo).

---

## **3\) S3 opcional (plugável)**

### **crates/ubl-storage/Cargo.toml**

\[features\]  
default \= \["fs"\]  
fs \= \[\]  
s3 \= \["aws-sdk-s3", "tokio"\]

\[dependencies\]  
async-trait \= "0.1"  
tokio \= { version \= "1", features \= \["fs", "macros", "rt-multi-thread"\] }  
aws-config \= { version \= "1", optional \= true }  
aws-sdk-s3 \= { version \= "1", optional \= true }

### **crates/ubl-storage/src/lib.rs**

\#\[async\_trait::async\_trait\]  
pub trait Storage: Send \+ Sync {  
    async fn put(\&self, key: String, bytes: Vec\<u8\>) \-\> anyhow::Result\<()\>;  
    async fn get(\&self, key: \&str) \-\> anyhow::Result\<Option\<Vec\<u8\>\>\>;  
    async fn list\_prefix(\&self, prefix: \&str) \-\> anyhow::Result\<Vec\<String\>\>;  
}

pub mod fs; // já existe  
\#\[cfg(feature="s3")\] pub mod s3;

### **crates/ubl-storage/src/s3.rs**

use aws\_sdk\_s3::{Client, primitives::ByteStream};  
use anyhow::Result;  
use super::Storage;

pub struct S3Store { client: Client, bucket: String, prefix: String }

impl S3Store {  
    pub async fn new(bucket: String, prefix: String) \-\> Result\<Self\> {  
        let conf \= aws\_config::load\_from\_env().await;  
        Ok(Self{ client: Client::new(\&conf), bucket, prefix })  
    }  
    fn key(\&self, k: \&str) \-\> String { format\!("{}/{}", self.prefix.trim\_end\_matches('/'), k) }  
}

\#\[async\_trait::async\_trait\]  
impl Storage for S3Store {  
    async fn put(\&self, key: String, bytes: Vec\<u8\>) \-\> Result\<()\> {  
        self.client.put\_object()  
            .bucket(\&self.bucket)  
            .key(self.key(\&key))  
            .body(ByteStream::from(bytes))  
            .send().await?;  
        Ok(())  
    }  
    async fn get(\&self, key: \&str) \-\> Result\<Option\<Vec\<u8\>\>\> {  
        let out \= self.client.get\_object()  
            .bucket(\&self.bucket)  
            .key(self.key(key))  
            .send().await;  
        match out {  
            Ok(o) \=\> Ok(Some(o.body.collect().await?.into\_bytes().to\_vec())),  
            Err(e) if is\_not\_found(\&e) \=\> Ok(None),  
            Err(e) \=\> Err(e.into()),  
        }  
    }  
    async fn list\_prefix(\&self, prefix: \&str) \-\> Result\<Vec\<String\>\> {  
        let p \= self.key(prefix);  
        let out \= self.client.list\_objects\_v2()  
            .bucket(\&self.bucket)  
            .prefix(p)  
            .send().await?;  
        Ok(out.contents().unwrap\_or\_default().iter()  
            .filter\_map(|o| o.key().map(|s| s.to\_string()))  
            .collect())  
    }  
}  
fn is\_not\_found(e: \&aws\_sdk\_s3::error::SdkError\<aws\_sdk\_s3::operation::get\_object::GetObjectError\>) \-\> bool {  
    matches\!(e, aws\_sdk\_s3::error::SdkError::ServiceError(se) if se.err().is\_no\_such\_key())  
}  
Env vars (S3):

* INVOICE\_S3\_BUCKET, INVOICE\_S3\_PREFIX → se definidos, Store::from\_env() usa S3.

* AWS creds padrão (AWS\_ACCESS\_KEY\_ID, AWS\_SECRET\_ACCESS\_KEY, AWS\_REGION).

---

## **4\) CI 🧪**

### **Acrescentar no job** 

### **registry-integration**

###  **(ci.yml):**

* Exportar CAPSULE\_KID, CAPSULE\_SK\_HEX, RECEIPT\_SK\_HEX.

* Exercitar fluxo: create quote → create invoice (ASK) → approve (ACK).

* Verificar que capsule e receipt apareceram na store FS temporária.

Exemplo (pseudostep):

export CAPSULE\_KID="did:ubl:ci\#k1"  
export CAPSULE\_SK\_HEX="$(python3 \- \<\<'PY'  
from nacl.signing import SigningKey; import binascii  
print(binascii.hexlify(SigningKey.generate().\_seed).decode())  
PY  
)"  
export RECEIPT\_SK\_HEX="$CAPSULE\_SK\_HEX"  
export INVOICE\_STORE\_DIR="$RUNNER\_TEMP/invoices"  
mkdir \-p "$INVOICE\_STORE\_DIR"

\# ... start registry on free port

qid=$(curl \-s localhost:$PORT/v1/quote/create \-H 'content-type: application/json' \-d '{"items":\[{"sku":"PLAN-PRO","qty":"1"}\]}' | jq \-r .id)  
curl \-s localhost:$PORT/v1/invoice/create\_from\_quote \-H 'content-type: application/json' \-d '{"quote\_id":"'"$qid"'","customer":{"id":"c1"},"currency":"USD","require\_ack":true}' | jq .  
iid=$(ls "$INVOICE\_STORE\_DIR"/invoices | head \-1 | sed 's/.json$//')  
curl \-s localhost:$PORT/v1/invoice/approve \-H 'content-type: application/json' \-d '{"id":"'"$iid"'"}' | jq .

test \-n "$(ls "$INVOICE\_STORE\_DIR"/capsules/invoice/"$iid"/\*.nrf 2\>/dev/null)" || (echo "missing capsule" && exit 1\)  
test \-n "$(ls "$INVOICE\_STORE\_DIR"/receipts/\*/\*.nrf 2\>/dev/null)" || (echo "missing receipt" && exit 1\)

---

## **5\) Operação 💼**

FS (default)

export INVOICE\_STORE\_DIR=/var/lib/ai-nrf1/invoices  
export CAPSULE\_KID=did:ubl:lab512\#k1  
export CAPSULE\_SK\_HEX=\<64-hex\>  
export RECEIPT\_SK\_HEX=\<64-hex\>  
make pm2-ai-restart

S3 (alternativo)

export INVOICE\_S3\_BUCKET=ubl-invoices  
export INVOICE\_S3\_PREFIX=prod/  
\# \+ AWS creds/region  
make pm2-ai-restart

---

## **6\) Aceite ✅**

* Approve/Reject salva invoice e grava cápsula UBL (capsules/invoice/\<invoice\_id\>/\<cap\_id\>.nrf).

* Hop receipt encadeado após a cápsula (receipts/\<cap\_id\>/\<hop\_id\>.nrf), com prev correto.

* FS funcionando por padrão; S3 funcionando quando habilitado.

* CI prova o fluxo E2E e checa a existência dos artefatos.

* Sem quebrar os módulos/rotas existentes.

---

Se quiser, na \#11 eu já trago:

* ubl verify para validar a cápsula/chain direto da store;

* Pre-signed URL S3 para baixar .nrf por 5 min;

* Webhook invoice-issued com retry/backoff idempotente.

Fechamos essa marreta? 💥🧰

