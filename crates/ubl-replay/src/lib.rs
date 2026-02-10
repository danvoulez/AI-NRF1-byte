use lru::LruCache;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Arc;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReplayError {
    #[error("Err.Replay.Replayed: duplicate (src, nonce) detected")]
    Replayed,
    #[error("Err.Replay.Expired: exp <= now")]
    Expired,
    #[error("Err.Replay.BadCapacity: capacity must be > 0")]
    BadCapacity,
}

#[derive(Clone)]
struct ReplayKey {
    src: String,
    nonce: [u8; 16],
}

impl PartialEq for ReplayKey {
    fn eq(&self, other: &Self) -> bool {
        self.src == other.src && self.nonce == other.nonce
    }
}
impl Eq for ReplayKey {}

impl Hash for ReplayKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.src.hash(state);
        self.nonce.hash(state);
    }
}

/// Anti-replay cache for `(hdr.src, hdr.nonce)` pairs.
///
/// - Not canonical: do NOT use inside the codec.
/// - TTL is derived from `hdr.exp - now` (capped by `max_ttl_ns`).
/// - Entries are considered expired when `now >= expires_at_ns`.
pub struct ReplayCache {
    cache: LruCache<ReplayKey, i64>,
    max_ttl_ns: i64,
    now_ns: Arc<dyn Fn() -> i64 + Send + Sync>,
}

impl ReplayCache {
    /// Create a new replay cache.
    ///
    /// - `capacity`: max number of entries kept (LRU).
    /// - `max_ttl_ns`: upper bound for TTL in nanoseconds (used if `exp` is far in the future, or absent).
    pub fn new(capacity: usize, max_ttl_ns: i64) -> Result<Self, ReplayError> {
        let cap = NonZeroUsize::new(capacity).ok_or(ReplayError::BadCapacity)?;
        Ok(Self {
            cache: LruCache::new(cap),
            max_ttl_ns,
            now_ns: Arc::new(crate::system_now_nanos_i64),
        })
    }

    /// Same as `new`, but inject a custom clock (for deterministic testing).
    pub fn with_now_fn(
        capacity: usize,
        max_ttl_ns: i64,
        now_ns: Arc<dyn Fn() -> i64 + Send + Sync>,
    ) -> Result<Self, ReplayError> {
        let cap = NonZeroUsize::new(capacity).ok_or(ReplayError::BadCapacity)?;
        Ok(Self {
            cache: LruCache::new(cap),
            max_ttl_ns,
            now_ns,
        })
    }

    /// Check and insert a `(src, nonce)` pair.
    ///
    /// Returns:
    /// - `Ok(())` if the pair was not seen (or was expired) and is inserted.
    /// - `Err(ReplayError::Replayed)` if the pair was already seen and not expired.
    /// - `Err(ReplayError::Expired)` if `exp <= now` (when `exp` is provided).
    pub fn check_and_insert(
        &mut self,
        src: &str,
        nonce: &[u8; 16],
        exp: Option<i64>,
    ) -> Result<(), ReplayError> {
        let now = (self.now_ns)();

        if let Some(exp) = exp {
            if exp <= now {
                return Err(ReplayError::Expired);
            }
        }

        let key = ReplayKey {
            src: src.to_string(),
            nonce: *nonce,
        };

        if let Some(expires_at) = self.cache.get(&key).copied() {
            if expires_at > now {
                return Err(ReplayError::Replayed);
            }
            // expired entry -> treat as absent and refresh
            self.cache.pop(&key);
        }

        let ttl_ns = match exp {
            Some(exp) => exp.saturating_sub(now),
            None => self.max_ttl_ns,
        };
        let ttl_ns = ttl_ns.clamp(0, self.max_ttl_ns);
        let expires_at = now.saturating_add(ttl_ns);

        self.cache.put(key, expires_at);
        Ok(())
    }
}

fn system_now_nanos_i64() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let nanos = (d.as_secs() as u128)
        .saturating_mul(1_000_000_000)
        .saturating_add(d.subsec_nanos() as u128);
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};

    #[test]
    fn new_nonce_ok_duplicate_detected() {
        let now = Arc::new(AtomicI64::new(100));
        let now_fn: Arc<dyn Fn() -> i64 + Send + Sync> = {
            let now = now.clone();
            Arc::new(move || now.load(Ordering::SeqCst))
        };

        let mut c = ReplayCache::with_now_fn(8, 1_000, now_fn).unwrap();
        let nonce = [0xAA; 16];
        assert!(c
            .check_and_insert("did:ubl:test:alice#key-1", &nonce, Some(200))
            .is_ok());
        assert_eq!(
            c.check_and_insert("did:ubl:test:alice#key-1", &nonce, Some(200))
                .unwrap_err(),
            ReplayError::Replayed
        );
    }

    #[test]
    fn expired_entry_allows_reinsert() {
        let now = Arc::new(AtomicI64::new(100));
        let now_fn: Arc<dyn Fn() -> i64 + Send + Sync> = {
            let now = now.clone();
            Arc::new(move || now.load(Ordering::SeqCst))
        };
        let mut c = ReplayCache::with_now_fn(8, 1_000, now_fn).unwrap();
        let nonce = [0xBB; 16];
        c.check_and_insert("did:ubl:test:alice#key-1", &nonce, Some(150))
            .unwrap();

        now.store(200, Ordering::SeqCst);
        // exp in past -> module returns Expired
        assert_eq!(
            c.check_and_insert("did:ubl:test:alice#key-1", &nonce, Some(150))
                .unwrap_err(),
            ReplayError::Expired
        );

        // but exp absent -> uses max ttl and can be inserted again (entry is expired)
        assert!(c
            .check_and_insert("did:ubl:test:alice#key-1", &nonce, None)
            .is_ok());
    }

    #[test]
    fn bad_capacity_rejected() {
        assert_eq!(
            ReplayCache::new(0, 1).err().unwrap(),
            ReplayError::BadCapacity
        );
    }
}
