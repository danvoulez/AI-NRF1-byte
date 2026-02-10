use std::net::SocketAddr;
use std::sync::OnceLock;

static INIT: OnceLock<()> = OnceLock::new();

pub fn ensure_exporter() {
    INIT.get_or_init(|| {
        if std::env::var("UBL_METRICS").ok().as_deref() != Some("1") {
            return;
        }

        let addr = std::env::var("UBL_METRICS_ADDR")
            .ok()
            .and_then(|s| s.parse::<SocketAddr>().ok())
            .unwrap_or_else(|| "0.0.0.0:9464".parse().expect("valid default addr"));

        let builder =
            metrics_exporter_prometheus::PrometheusBuilder::new().with_http_listener(addr);
        let _ = builder.install(); // best-effort
    });
}
