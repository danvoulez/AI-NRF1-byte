//! Product manifest types (design doc §5).

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct Manifest {
    pub v: String,
    pub name: String,
    pub version: String,
    pub pipeline: Vec<Step>,
    #[serde(default)]
    pub io_bindings: Option<Value>,
}

#[derive(Deserialize, Debug)]
pub struct Step {
    pub step_id: String,
    pub kind: String,
    pub version: String,
    pub config: Value,
    #[serde(default)]
    pub on_error: Option<String>,
    #[serde(default, rename = "if")]
    pub cond: Option<String>,
}
