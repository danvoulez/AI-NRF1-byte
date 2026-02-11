//! Durable idempotency store — file-based KV in `STATE_DIR/idem/<run_key>`.
//!
//! Each entry is a small marker file. TTL is checked by mtime.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// File-based idempotency store.
pub struct IdempotencyStore {
    base_dir: PathBuf,
    ttl: Duration,
}

impl IdempotencyStore {
    pub fn new(base_dir: impl Into<PathBuf>, ttl: Duration) -> Self {
        Self {
            base_dir: base_dir.into(),
            ttl,
        }
    }

    fn key_path(&self, run_key: &str) -> PathBuf {
        self.base_dir.join(run_key)
    }

    /// Check if a run_key has already been executed (within TTL).
    pub fn contains(&self, run_key: &str) -> bool {
        let path = self.key_path(run_key);
        if !path.exists() {
            return false;
        }
        // Check TTL by mtime
        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                if let Ok(age) = SystemTime::now().duration_since(modified) {
                    if age > self.ttl {
                        // Expired — remove and return false
                        let _ = std::fs::remove_file(&path);
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Mark a run_key as executed.
    pub fn mark(&self, run_key: &str) -> anyhow::Result<()> {
        let path = self.key_path(run_key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, b"")?;
        Ok(())
    }

    /// Garbage-collect expired entries.
    pub fn gc(&self) -> anyhow::Result<usize> {
        let mut removed = 0;
        if !self.base_dir.exists() {
            return Ok(0);
        }
        for entry in std::fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(age) = SystemTime::now().duration_since(modified) {
                        if age > self.ttl {
                            let _ = std::fs::remove_file(entry.path());
                            removed += 1;
                        }
                    }
                }
            }
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_and_contains() {
        let dir = std::env::temp_dir().join("ai-nrf1-test-idem");
        let _ = std::fs::remove_dir_all(&dir);
        let store = IdempotencyStore::new(&dir, Duration::from_secs(3600));

        assert!(!store.contains("key1"));
        store.mark("key1").unwrap();
        assert!(store.contains("key1"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expired_entry_removed() {
        let dir = std::env::temp_dir().join("ai-nrf1-test-idem-ttl");
        let _ = std::fs::remove_dir_all(&dir);
        // TTL of 0 means everything is expired immediately
        let store = IdempotencyStore::new(&dir, Duration::from_secs(0));

        store.mark("key2").unwrap();
        // Sleep a tiny bit so mtime is in the past
        std::thread::sleep(Duration::from_millis(10));
        assert!(!store.contains("key2"), "should be expired");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
