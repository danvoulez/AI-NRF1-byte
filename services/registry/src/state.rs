use std::sync::Arc;
use ed25519_dalek::SigningKey;
use ubl_storage::ledger::LedgerWriter;

// ---------------------------------------------------------------------------
// AppState — the chassis that any product mounts on (BASE terrain)
//
// Shared resources: signing key, runtime attestation, ledger, config.
// No database. Persistence is through the LedgerWriter trait (MODULE).
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Config {
    pub public_base: String,        // e.g. "https://passports.ubl.agency"
    pub issuer_did: String,         // DID of this service instance
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            public_base: std::env::var("CDN_BASE")
                .unwrap_or_else(|_| "https://passports.ubl.agency".into()),
            issuer_did: std::env::var("ISSUER_DID")
                .unwrap_or_else(|_| "did:ubl:registry-dev".into()),
        }
    }
}

pub struct AppState {
    pub cfg: Config,
    pub signing_key: SigningKey,
    pub verifying_key: ed25519_dalek::VerifyingKey,
    pub runtime: runtime::SelfAttestation,
    pub ledger: Arc<dyn LedgerWriter>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Arc<Self>> {
        let cfg = Config::from_env();

        // Signing key: load from SIGNING_KEY_HEX env var, or generate for dev
        let signing_key = match std::env::var("SIGNING_KEY_HEX") {
            Ok(hex_str) => {
                let bytes = hex::decode(&hex_str)?;
                let bytes: [u8; 32] = bytes.try_into()
                    .map_err(|_| anyhow::anyhow!("SIGNING_KEY_HEX must be 32 bytes (64 hex chars)"))?;
                SigningKey::from_bytes(&bytes)
            }
            Err(_) => {
                tracing::warn!("SIGNING_KEY_HEX not set — generating ephemeral key (dev mode)");
                let mut rng = rand::thread_rng();
                SigningKey::generate(&mut rng)
            }
        };
        let verifying_key = signing_key.verifying_key();

        // Runtime attestation
        let binary_sha256 = std::env::var("BINARY_SHA256")
            .unwrap_or_else(|_| "dev-build-no-hash".into());
        let rt = runtime::SelfAttestation::new(&binary_sha256);

        // Append-only audit ledger (BASE trait, MODULE implementation)
        #[cfg(feature = "module-ledger-ndjson")]
        let ledger: Arc<dyn LedgerWriter> = {
            let l = Arc::new(ledger_ndjson::NdjsonLedger::from_env());
            tracing::info!("ledger: ndjson ({})", std::env::var("LEDGER_DIR").unwrap_or_else(|_| "./data/ledger".into()));
            l
        };
        #[cfg(not(feature = "module-ledger-ndjson"))]
        let ledger: Arc<dyn LedgerWriter> = {
            tracing::info!("ledger: null (no ledger module compiled in)");
            Arc::new(ubl_storage::ledger::NullLedger)
        };

        Ok(Arc::new(Self {
            cfg,
            signing_key,
            verifying_key,
            runtime: rt,
            ledger,
        }))
    }
}
