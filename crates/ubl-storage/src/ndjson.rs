use crate::ledger::{LedgerEntry, LedgerError, LedgerWriter};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// NdjsonLedger — file-based append-only NDJSON ledger (ρ-canonical JSON)
//
// Layout:
//   {base_dir}/{app}/{tenant}/{stream}.ndjson          (active)
//   {base_dir}/{app}/{tenant}/{stream}.2026-W07.ndjson.gz  (compressed weekly)
//
// Each line is a ρ-canonical JSON object (ai-json-nrf1 view).
// Serialization path: LedgerEntry → nrf_core::Value → ρ-normalize → to_json.
// Deserialization path: JSON line → from_canonical_json (validates NRF round-trip).
// Files are append-only. No line is ever modified or deleted.
// Weekly compression rotates the active file into a gzipped archive.
// ---------------------------------------------------------------------------

pub struct NdjsonLedger {
    base_dir: PathBuf,
}

impl NdjsonLedger {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        tracing::info!(base_dir = %base_dir.display(), "ndjson ledger initialized");
        Self { base_dir }
    }

    /// Path to the active NDJSON file for a given entry
    fn active_path(&self, entry: &LedgerEntry) -> PathBuf {
        self.base_dir
            .join(&entry.app)
            .join(&entry.tenant)
            .join(format!("{}.ndjson", entry.stream_name()))
    }

    /// Path to the active NDJSON file for a given (app, tenant, stream)
    fn stream_path(&self, app: &str, tenant: &str, stream: &str) -> PathBuf {
        self.base_dir
            .join(app)
            .join(tenant)
            .join(format!("{stream}.ndjson"))
    }

    /// Read all entries from a stream, most recent last.
    /// Reads both the active file and any compressed archives.
    pub fn read_stream(
        &self,
        app: &str,
        tenant: &str,
        stream: &str,
    ) -> Result<Vec<LedgerEntry>, LedgerError> {
        let mut entries = Vec::new();

        // Read compressed archives first (older data)
        let dir = self.base_dir.join(app).join(tenant);
        if dir.exists() {
            let prefix = format!("{stream}.");
            let mut archives: Vec<_> = std::fs::read_dir(&dir)
                .map_err(|e| LedgerError::Io(e.to_string()))?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.starts_with(&prefix) && name.ends_with(".ndjson.gz")
                })
                .collect();
            archives.sort_by_key(|e| e.file_name());

            for archive in archives {
                let file = std::fs::File::open(archive.path())
                    .map_err(|e| LedgerError::Io(e.to_string()))?;
                let decoder = flate2::read::GzDecoder::new(file);
                let reader = std::io::BufReader::new(decoder);
                for line in std::io::BufRead::lines(reader) {
                    let line = line.map_err(|e| LedgerError::Io(e.to_string()))?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    match LedgerEntry::from_canonical_json(&line) {
                        Ok(entry) => entries.push(entry),
                        Err(e) => {
                            tracing::warn!(error = %e, "skipping malformed ledger line in archive");
                        }
                    }
                }
            }
        }

        // Read active file (newest data)
        let active = self.stream_path(app, tenant, stream);
        if active.exists() {
            let file = std::fs::File::open(&active)
                .map_err(|e| LedgerError::Io(e.to_string()))?;
            let reader = std::io::BufReader::new(file);
            for line in std::io::BufRead::lines(reader) {
                let line = line.map_err(|e| LedgerError::Io(e.to_string()))?;
                if line.trim().is_empty() {
                    continue;
                }
                match LedgerEntry::from_canonical_json(&line) {
                    Ok(entry) => entries.push(entry),
                    Err(e) => {
                        tracing::warn!(error = %e, "skipping malformed ledger line");
                    }
                }
            }
        }

        Ok(entries)
    }

    /// Read the last N entries from a stream (most recent first).
    pub fn read_stream_tail(
        &self,
        app: &str,
        tenant: &str,
        stream: &str,
        limit: usize,
    ) -> Result<Vec<LedgerEntry>, LedgerError> {
        let all = self.read_stream(app, tenant, stream)?;
        let start = all.len().saturating_sub(limit);
        let mut tail: Vec<LedgerEntry> = all[start..].to_vec();
        tail.reverse();
        Ok(tail)
    }

    /// Compress the active file for a given (app, tenant, stream) into a weekly archive.
    /// The active file is replaced with an empty file.
    ///
    /// Archive name: {stream}.{iso_week}.ndjson.gz
    /// Example: executions.2026-W07.ndjson.gz
    pub fn compress_weekly(
        &self,
        app: &str,
        tenant: &str,
        stream: &str,
    ) -> Result<Option<PathBuf>, LedgerError> {
        let active = self.stream_path(app, tenant, stream);
        if !active.exists() {
            return Ok(None);
        }

        let metadata = std::fs::metadata(&active)
            .map_err(|e| LedgerError::Io(e.to_string()))?;
        if metadata.len() == 0 {
            return Ok(None);
        }

        // Determine ISO week for archive name
        let now = chrono::Utc::now();
        let iso_week = now.format("%G-W%V").to_string();
        let archive_name = format!("{stream}.{iso_week}.ndjson.gz");
        let archive_path = self.base_dir.join(app).join(tenant).join(&archive_name);

        // Read active file content
        let content = std::fs::read(&active)
            .map_err(|e| LedgerError::Io(e.to_string()))?;

        // Compress into gzip
        let out_file = std::fs::File::create(&archive_path)
            .map_err(|e| LedgerError::Io(e.to_string()))?;
        let mut encoder = GzEncoder::new(out_file, Compression::default());
        encoder
            .write_all(&content)
            .map_err(|e| LedgerError::Io(e.to_string()))?;
        encoder
            .finish()
            .map_err(|e| LedgerError::Io(e.to_string()))?;

        // Truncate active file
        std::fs::write(&active, b"")
            .map_err(|e| LedgerError::Io(e.to_string()))?;

        let original_size = content.len();
        let compressed_size = std::fs::metadata(&archive_path)
            .map(|m| m.len())
            .unwrap_or(0);
        tracing::info!(
            app = app,
            tenant = tenant,
            stream = stream,
            archive = %archive_name,
            original_bytes = original_size,
            compressed_bytes = compressed_size,
            "weekly compression complete"
        );

        Ok(Some(archive_path))
    }

    /// Compress all active streams for all tenants/apps.
    /// Call this on a weekly schedule (e.g. via cron or tokio interval).
    pub fn compress_all(&self) -> Result<Vec<PathBuf>, LedgerError> {
        let mut compressed = Vec::new();

        if !self.base_dir.exists() {
            return Ok(compressed);
        }

        for app_entry in std::fs::read_dir(&self.base_dir)
            .map_err(|e| LedgerError::Io(e.to_string()))?
        {
            let app_entry = app_entry.map_err(|e| LedgerError::Io(e.to_string()))?;
            if !app_entry.file_type().map_or(false, |t| t.is_dir()) {
                continue;
            }
            let app = app_entry.file_name().to_string_lossy().to_string();

            for tenant_entry in std::fs::read_dir(app_entry.path())
                .map_err(|e| LedgerError::Io(e.to_string()))?
            {
                let tenant_entry = tenant_entry.map_err(|e| LedgerError::Io(e.to_string()))?;
                if !tenant_entry.file_type().map_or(false, |t| t.is_dir()) {
                    continue;
                }
                let tenant = tenant_entry.file_name().to_string_lossy().to_string();

                for stream in &["executions", "receipts", "ghosts"] {
                    if let Some(path) = self.compress_weekly(&app, &tenant, stream)? {
                        compressed.push(path);
                    }
                }
            }
        }

        Ok(compressed)
    }

    /// List all (app, tenant) pairs that have ledger data.
    pub fn list_partitions(&self) -> Result<Vec<(String, String)>, LedgerError> {
        let mut partitions = Vec::new();
        if !self.base_dir.exists() {
            return Ok(partitions);
        }
        for app_entry in std::fs::read_dir(&self.base_dir)
            .map_err(|e| LedgerError::Io(e.to_string()))?
        {
            let app_entry = app_entry.map_err(|e| LedgerError::Io(e.to_string()))?;
            if !app_entry.file_type().map_or(false, |t| t.is_dir()) {
                continue;
            }
            let app = app_entry.file_name().to_string_lossy().to_string();
            for tenant_entry in std::fs::read_dir(app_entry.path())
                .map_err(|e| LedgerError::Io(e.to_string()))?
            {
                let tenant_entry = tenant_entry.map_err(|e| LedgerError::Io(e.to_string()))?;
                if !tenant_entry.file_type().map_or(false, |t| t.is_dir()) {
                    continue;
                }
                let tenant = tenant_entry.file_name().to_string_lossy().to_string();
                partitions.push((app.clone(), tenant));
            }
        }
        Ok(partitions)
    }
}

#[async_trait::async_trait]
impl LedgerWriter for NdjsonLedger {
    async fn append(&self, entry: &LedgerEntry) -> Result<(), LedgerError> {
        let path = self.active_path(entry);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Serialize to ρ-canonical JSON line (NRF → ρ-normalize → ai-json-nrf1)
        let mut line = entry.to_canonical_json()?;
        line.push('\n');

        // Append to file (atomic per-line via O_APPEND)
        tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?
            .write_all(line.as_bytes())
            .await?;

        Ok(())
    }
}

// We need tokio::io::AsyncWriteExt for write_all
use tokio::io::AsyncWriteExt;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{LedgerEntry, LedgerEvent};
    use uuid::Uuid;

    fn test_entry(app: &str, tenant: &str) -> LedgerEntry {
        LedgerEntry::now(
            LedgerEvent::PipelineExecuted,
            app,
            tenant,
            None,
            vec![],
            Uuid::nil(),
            "b3:test",
            "did:test",
            Some("ALLOW".into()),
            serde_json::json!({"test": true}),
        )
    }

    #[tokio::test]
    async fn test_append_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = NdjsonLedger::new(dir.path());

        let entry = test_entry("myapp", "default");
        ledger.append(&entry).await.unwrap();
        ledger.append(&entry).await.unwrap();

        let entries = ledger.read_stream("myapp", "default", "executions").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].app, "myapp");
        assert_eq!(entries[0].tenant, "default");
    }

    #[tokio::test]
    async fn test_compress_weekly() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = NdjsonLedger::new(dir.path());

        // Write some entries
        for _ in 0..5 {
            ledger.append(&test_entry("myapp", "t1")).await.unwrap();
        }

        // Compress
        let result = ledger.compress_weekly("myapp", "t1", "executions").unwrap();
        assert!(result.is_some());

        // Active file should be empty now
        let active = ledger.stream_path("myapp", "t1", "executions");
        assert_eq!(std::fs::read_to_string(&active).unwrap(), "");

        // But reading stream should still return entries from archive
        let entries = ledger.read_stream("myapp", "t1", "executions").unwrap();
        assert_eq!(entries.len(), 5);
    }

    #[tokio::test]
    async fn test_canonical_round_trip() {
        let entry = test_entry("myapp", "default");

        // Serialize to canonical JSON
        let json_line = entry.to_canonical_json().unwrap();

        // Verify it's valid JSON with sorted keys (ρ-canonical BTreeMap)
        let parsed: serde_json::Value = serde_json::from_str(&json_line).unwrap();
        assert!(parsed.is_object());
        let obj = parsed.as_object().unwrap();
        // Keys must be sorted (BTreeMap guarantee from ρ-normalize)
        let keys: Vec<&String> = obj.keys().collect();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        assert_eq!(keys, sorted_keys, "keys must be ρ-sorted");

        // Verify NRF round-trip: JSON → NRF Value → JSON should be identical
        let nrf_val = ubl_json_view::from_json(&parsed).unwrap();
        let back_json = ubl_json_view::to_json(&nrf_val);
        assert_eq!(parsed, back_json, "canonical JSON must survive NRF round-trip");

        // Deserialize back to LedgerEntry
        let restored = LedgerEntry::from_canonical_json(&json_line).unwrap();
        assert_eq!(restored.app, "myapp");
        assert_eq!(restored.tenant, "default");
        assert_eq!(restored.decision, Some("ALLOW".into()));

        // Verify idempotent: re-serialize must produce identical bytes
        let json_line_2 = restored.to_canonical_json().unwrap();
        assert_eq!(json_line, json_line_2, "canonical serialization must be idempotent");
    }

    #[tokio::test]
    async fn test_read_tail() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = NdjsonLedger::new(dir.path());

        for _ in 0..10 {
            ledger.append(&test_entry("app", "t")).await.unwrap();
        }

        let tail = ledger.read_stream_tail("app", "t", "executions", 3).unwrap();
        assert_eq!(tail.len(), 3);
    }
}
