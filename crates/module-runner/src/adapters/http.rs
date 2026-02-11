//! HTTP adapter for Webhook and RelayOut effects.
//!
//! - Timeout 5s, exponential retries (max 5)
//! - Idempotency-Key header: `b3(capsule_id || step_id || discriminator)`
//! - Optional HMAC: `X-UBL-Signature: sha256=<hex(hmac(secret, body))>`

#[cfg(feature = "live")]
use hmac::{Hmac, Mac};
#[cfg(feature = "live")]
use sha2::Sha256;

/// Outcome of an HTTP POST attempt.
#[derive(Debug)]
pub struct HttpOutcome {
    pub status: u16,
    pub retries: u32,
    pub latency_ms: u64,
}

/// HTTP client with retry + HMAC support.
#[cfg(feature = "live")]
pub struct HttpClient {
    inner: reqwest::Client,
    max_retries: u32,
}

#[cfg(feature = "live")]
impl HttpClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("failed to build reqwest client");
        Self {
            inner: client,
            max_retries: 5,
        }
    }

    pub fn with_max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    /// POST with retries, idempotency key, and optional HMAC.
    pub async fn post(
        &self,
        url: &str,
        body: &[u8],
        content_type: &str,
        hmac_secret: Option<&str>,
        idem_key: &str,
    ) -> anyhow::Result<HttpOutcome> {
        let t0 = std::time::Instant::now();
        let mut retries = 0u32;

        loop {
            let mut req = self
                .inner
                .post(url)
                .header("Content-Type", content_type)
                .header("Idempotency-Key", idem_key)
                .body(body.to_vec());

            if let Some(secret) = hmac_secret {
                let sig = compute_hmac(secret.as_bytes(), body);
                req = req.header("X-UBL-Signature", format!("sha256={}", sig));
            }

            match req.send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let latency_ms = t0.elapsed().as_millis() as u64;

                    tracing::info!(
                        url = %url,
                        status = status,
                        retries = retries,
                        latency_ms = latency_ms,
                        "http.post"
                    );

                    if status >= 500 && retries < self.max_retries {
                        retries += 1;
                        let delay = std::time::Duration::from_millis(100 * 2u64.pow(retries));
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    return Ok(HttpOutcome {
                        status,
                        retries,
                        latency_ms,
                    });
                }
                Err(e) => {
                    if retries < self.max_retries {
                        retries += 1;
                        tracing::warn!(
                            url = %url,
                            error = %e,
                            retry = retries,
                            "http.post.retry"
                        );
                        let delay = std::time::Duration::from_millis(100 * 2u64.pow(retries));
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(anyhow::anyhow!(
                        "http.post failed after {} retries: {}",
                        retries,
                        e
                    ));
                }
            }
        }
    }
}

/// Compute HMAC-SHA256 and return hex string.
#[cfg(feature = "live")]
fn compute_hmac(key: &[u8], body: &[u8]) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key length");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

/// Compute idempotency key from capsule_id + step_id + discriminator.
pub fn idempotency_key(capsule_id_hex: &str, step_id: &str, discriminator: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(capsule_id_hex.as_bytes());
    hasher.update(step_id.as_bytes());
    hasher.update(discriminator.as_bytes());
    hex::encode(hasher.finalize().as_bytes())
}
