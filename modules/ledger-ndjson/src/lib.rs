use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use ubl_storage::ledger::{LedgerEntry, LedgerError, LedgerWriter};

// ---------------------------------------------------------------------------
// ledger-ndjson — MODULE implementation of LedgerWriter
//
// Append-only NDJSON files on local filesystem.
//
// File layout: {base_dir}/{app}/{tenant}/{stream}.ndjson
// where stream = "receipts" | "ghosts" (from LedgerEntry::stream_name()).
//
// Thread-safe: Mutex serializes writes. Each write is a single JSON line
// followed by newline, flushed immediately. No partial writes.
//
// The BASE defines the trait (ubl-storage::ledger::LedgerWriter).
// This module defines how: local filesystem, NDJSON format.
// ---------------------------------------------------------------------------

pub struct NdjsonLedger {
    base_dir: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl NdjsonLedger {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn from_env() -> Self {
        let dir = std::env::var("LEDGER_DIR").unwrap_or_else(|_| "./data/ledger".into());
        Self::new(dir)
    }
}

#[async_trait::async_trait]
impl LedgerWriter for NdjsonLedger {
    async fn append(&self, entry: &LedgerEntry) -> Result<(), LedgerError> {
        use tokio::io::AsyncWriteExt;

        // Serialize to a single line (no pretty-print)
        let mut line = serde_json::to_string(entry)?;
        line.push('\n');

        // Build path: {base_dir}/{app}/{tenant}/{stream}.ndjson
        let dir = self.base_dir.join(&entry.app).join(&entry.tenant);
        let file_path = dir.join(format!("{}.ndjson", entry.stream_name()));

        // Atomic append under lock
        let _guard = self.lock.lock().await;
        tokio::fs::create_dir_all(&dir).await?;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;
        file.write_all(line.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }
}
