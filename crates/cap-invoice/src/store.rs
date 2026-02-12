use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

/// Simple filesystem-based JSON store for invoices.
pub struct FsStore {
    root: PathBuf,
}

impl FsStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn from_env() -> Self {
        let root = std::env::var("INVOICE_STORE_DIR")
            .unwrap_or_else(|_| "./data/invoices".into());
        let _ = std::fs::create_dir_all(&root);
        Self::new(root)
    }

    pub fn put_json<T: Serialize>(&self, key: &str, v: &T) -> Result<()> {
        let path = self.root.join(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(v)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub fn get_json<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let path = self.root.join(key);
        if !path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(path)?;
        Ok(Some(serde_json::from_slice(&bytes)?))
    }
}
