use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

// ---------------------------------------------------------------------------
// Product Identity — extracted from X-Tenant + X-Product headers
//
// Required on all /api/v0/* routes. Inserted into request extensions
// so handlers can access it via Extension<ProductIdentity>.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ProductIdentity {
    pub tenant: String,
    pub product: String,
}

/// Middleware that requires X-Tenant and X-Product headers.
/// Returns 400 Bad Request if either is missing or empty.
pub async fn require_identity(
    mut req: Request,
    next: Next,
) -> Response {
    let tenant = req
        .headers()
        .get("x-tenant")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let product = req
        .headers()
        .get("x-product")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    match (tenant, product) {
        (Some(tenant), Some(product)) => {
            req.extensions_mut().insert(ProductIdentity {
                tenant,
                product,
            });
            next.run(req).await
        }
        (None, _) => (
            StatusCode::BAD_REQUEST,
            "missing required header: X-Tenant",
        )
            .into_response(),
        (_, None) => (
            StatusCode::BAD_REQUEST,
            "missing required header: X-Product",
        )
            .into_response(),
    }
}
