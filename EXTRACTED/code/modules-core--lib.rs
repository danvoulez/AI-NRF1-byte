use serde::{Deserialize, Serialize};

pub type Cid = [u8; 32];

#[derive(Clone, Debug)]
pub struct ExecutionMeta {
    pub run_id: String,
    pub tenant: Option<String>,
    pub trace_id: Option<String>,
    pub ts_nanos: i64,
}

#[derive(Clone, Debug)]
pub struct Artifact {
    pub cid: Option<Cid>,
    pub mime: String,
    pub bytes: Vec<u8>,
    pub name: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Effect {
    Webhook { url: String, body: Vec<u8>, content_type: String, hmac_key_env: Option<String> },
    WriteStorage { path: String, bytes: Vec<u8>, mime: String },
    // … adicione depois
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    Allow,
    Deny,
    Require,
}

#[derive(Clone, Debug)]
pub struct CapInput {
    pub env: ai_nrf1::Value,         // Canon
    pub config: serde_json::Value,   // Manifesto do produto
    pub assets: Box<dyn AssetResolver>,
    pub prev_receipts: Vec<Cid>,
    pub meta: ExecutionMeta,
}

#[derive(Clone, Debug, Default)]
pub struct CapOutput {
    pub new_env: Option<ai_nrf1::Value>,
    pub verdict: Option<Verdict>,
    pub artifacts: Vec<Artifact>,
    pub effects: Vec<Effect>,
    pub metrics: Vec<(String, i64)>, // chave, valor
}

#[async_trait::async_trait]
pub trait AssetResolver: Send + Sync {
    fn get(&self, cid: &Cid) -> anyhow::Result<Asset>;
    fn box_clone(&self) -> Box<dyn AssetResolver>;
}

#[derive(Clone)]
pub struct Asset { pub cid: Cid, pub bytes: Vec<u8>, pub mime: String }

impl std::fmt::Debug for dyn AssetResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "AssetResolver(..)") }
}

pub trait Capability: Send + Sync {
    /// Identidade do módulo
    const KIND: &'static str;
    const API_VERSION: &'static str;

    /// Valida o fragmento de configuração do manifesto para este módulo
    fn validate_config(&self, config: &serde_json::Value) -> anyhow::Result<()>;

    /// Execução pura (determinística): sem IO, sem rede, sem DB
    fn execute(&self, input: CapInput) -> anyhow::Result<CapOutput>;
}
