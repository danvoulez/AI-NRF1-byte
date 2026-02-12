use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Per-product rate limiter — token bucket
//
// Configured via RATE_LIMIT_RPM env var (requests per minute per product).
// Default: 120 rpm. Set to 0 to disable.
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct RateLimiter {
    rpm: u32,
    buckets: Arc<Mutex<HashMap<String, Bucket>>>,
}

struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn from_env() -> Self {
        let rpm: u32 = std::env::var("RATE_LIMIT_RPM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(120);
        if rpm == 0 {
            tracing::info!("rate limiting disabled (RATE_LIMIT_RPM=0)");
        } else {
            tracing::info!(rpm = rpm, "rate limiter initialized");
        }
        Self {
            rpm,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn try_acquire(&self, product: &str) -> bool {
        if self.rpm == 0 {
            return true;
        }
        let mut buckets = self.buckets.lock().unwrap();
        let now = Instant::now();
        let bucket = buckets.entry(product.to_string()).or_insert(Bucket {
            tokens: self.rpm as f64,
            last_refill: now,
        });

        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        let refill = elapsed * (self.rpm as f64 / 60.0);
        bucket.tokens = (bucket.tokens + refill).min(self.rpm as f64);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Middleware that enforces per-product rate limits.
/// Requires ProductIdentity in extensions (run after require_identity).
pub async fn rate_limit(
    req: Request,
    next: Next,
) -> Response {
    let limiter = req.extensions().get::<Arc<RateLimiter>>().cloned();

    let limiter = match limiter {
        Some(l) if l.rpm > 0 => l,
        _ => return next.run(req).await, // disabled or not configured
    };

    let product = req
        .extensions()
        .get::<super::identity::ProductIdentity>()
        .map(|id| id.product.clone())
        .unwrap_or_default();

    if limiter.try_acquire(&product) {
        next.run(req).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            "rate limit exceeded for this product",
        )
            .into_response()
    }
}
