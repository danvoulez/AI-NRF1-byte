//! Receipt signer adapter — resolves signing key from bindings,
//! calls `ubl_capsule::receipt::sign_receipt`, and verifies chain post-append.

use ubl_capsule::types::Receipt;

#[cfg(feature = "live")]
use ubl_capsule::types::Capsule;

/// Draft for a new hop receipt (before signing).
#[derive(Debug, Clone)]
pub struct HopDraft {
    pub capsule_id: [u8; 32],
    pub prev: [u8; 32],
    pub kind: String,
    pub node: String,
    pub ts: i64,
}

/// Trait for receipt signing — allows test doubles.
pub trait ReceiptSigner: Send + Sync {
    fn sign_hop(&self, draft: &HopDraft) -> anyhow::Result<Receipt>;
}

/// Ed25519 signer using a raw signing key.
#[cfg(feature = "live")]
pub struct Ed25519ReceiptSigner {
    sk: ed25519_dalek::SigningKey,
}

#[cfg(feature = "live")]
impl Ed25519ReceiptSigner {
    /// Create from raw 32-byte seed.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        Self {
            sk: ed25519_dalek::SigningKey::from_bytes(seed),
        }
    }

    /// Create from hex-encoded seed (64 hex chars = 32 bytes).
    pub fn from_hex(hex_seed: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(hex_seed)?;
        let seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("seed must be 32 bytes"))?;
        Ok(Self::from_seed(&seed))
    }
}

#[cfg(feature = "live")]
impl ReceiptSigner for Ed25519ReceiptSigner {
    fn sign_hop(&self, draft: &HopDraft) -> anyhow::Result<Receipt> {
        ubl_capsule::receipt::add_hop(
            draft.capsule_id,
            draft.prev,
            &draft.kind,
            &draft.node,
            draft.ts,
            &self.sk,
        )
        .map_err(|e| anyhow::anyhow!("receipt sign failed: {}", e))
    }
}

/// Noop signer for tests — returns a receipt with zeroed sig.
pub struct NoopSigner;

impl ReceiptSigner for NoopSigner {
    fn sign_hop(&self, draft: &HopDraft) -> anyhow::Result<Receipt> {
        Ok(Receipt {
            id: [0u8; 32],
            of: draft.capsule_id,
            prev: draft.prev,
            kind: draft.kind.clone(),
            node: draft.node.clone(),
            ts: draft.ts,
            sig: [0u8; 64],
        })
    }
}

/// Append a signed hop to a capsule and verify the chain.
#[cfg(feature = "live")]
pub fn append_and_verify(
    capsule: &mut Capsule,
    signer: &dyn ReceiptSigner,
    kind: &str,
    node: &str,
    ts: i64,
    resolve_pk: &dyn Fn(&str) -> Option<ed25519_dalek::VerifyingKey>,
) -> anyhow::Result<()> {
    let prev = capsule
        .receipts
        .last()
        .map(|r| r.id)
        .unwrap_or([0u8; 32]);

    let draft = HopDraft {
        capsule_id: capsule.id,
        prev,
        kind: kind.into(),
        node: node.into(),
        ts,
    };

    let receipt = signer.sign_hop(&draft)?;
    capsule.receipts.push(receipt);

    ubl_capsule::receipt::verify_chain(&capsule.id, &capsule.receipts, resolve_pk)
        .map_err(|e| anyhow::anyhow!("chain verify failed after append: {}", e))
}
