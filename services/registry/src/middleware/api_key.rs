use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// API Key Auth — per-product key validation
//
// Keys are loaded from API_KEYS env var as comma-separated product:key pairs.
// Example: API_KEYS=tdln:sk_abc123,acme-verify:sk_def456
//
// If API_KEYS is not set, all requests are allowed (dev mode).
// If set, X-API-Key header must match the key for the product in X-Product.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ApiKeyStore {
    /// product_slug → api_key
    keys: HashMap<String, String>,
    /// If true, skip validation (dev mode when no keys configured)
    pub allow_all: bool,
}

impl ApiKeyStore {
    /// Load from API_KEYS env var. Returns allow_all=true if not set.
    pub fn from_env() -> Self {
        match std::env::var("API_KEYS") {
            Ok(val) if !val.trim().is_empty() => {
                let mut keys = HashMap::new();
                for pair in val.split(',') {
                    let pair = pair.trim();
                    if let Some((product, key)) = pair.split_once(':') {
                        keys.insert(product.to_string(), key.to_string());
                    }
                }
                tracing::info!(products = keys.len(), "api key store loaded");
                Self {
                    keys,
                    allow_all: false,
                }
            }
            _ => {
                tracing::info!("API_KEYS not set — dev mode (all requests allowed)");
                Self {
                    keys: HashMap::new(),
                    allow_all: true,
                }
            }
        }
    }

    pub fn validate(&self, product: &str, key: &str) -> bool {
        self.keys.get(product).map_or(false, |k| k == key)
    }
}

/// Middleware that validates X-API-Key against the product's registered key.
/// Requires ProductIdentity to already be in extensions (run after require_identity).
pub async fn require_api_key(
    req: Request,
    next: Next,
) -> Response {
    let store = req
        .extensions()
        .get::<Arc<ApiKeyStore>>()
        .cloned();

    let store = match store {
        Some(s) => s,
        None => return next.run(req).await, // no store = dev mode
    };

    if store.allow_all {
        return next.run(req).await;
    }

    let identity = req
        .extensions()
        .get::<super::identity::ProductIdentity>()
        .cloned();

    let product = match identity {
        Some(id) => id.product,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                "missing product identity (run require_identity first)",
            )
                .into_response();
        }
    };

    let api_key = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if store.validate(&product, api_key) {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "invalid or missing X-API-Key for this product",
        )
            .into_response()
    }
}
