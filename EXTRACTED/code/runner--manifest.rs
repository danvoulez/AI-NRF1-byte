use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct Manifest {
    pub v: String,            // "product-v1"
    pub name: String,
    pub version: String,
    pub pipeline: Vec<Step>,
    #[serde(default)]
    pub io_bindings: Option<Value>,
}

#[derive(Deserialize, Debug)]
pub struct Step {
    pub step_id: String,
    pub kind: String,         // "cap-intake", "cap-policy", etc.
    pub version: String,      // "^1.0"
    pub config: Value,
    #[serde(default)]
    pub on_error: Option<String>, // "nack" | "skip" | "fail"
    #[serde(default, rename = "if")]
    pub cond: Option<String>,     // reservado; parser simples depois
}
