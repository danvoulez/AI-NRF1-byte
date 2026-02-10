//! Core types for UBL Capsule v1.

use serde::{Deserialize, Serialize};

pub const DOMAIN: &str = "ubl-capsule/1.0";
pub const RECEIPT_DOMAIN: &str = "ubl-receipt/1.0";

/// A complete UBL Capsule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capsule {
    pub domain: String,
    /// Stable content-address — blake3(nrf.encode(capsule \ {id, seal.sig, receipts[*].sig}))
    #[serde(with = "hex_bytes_32")]
    pub id: [u8; 32],
    pub hdr: Header,
    pub env: Envelope,
    pub seal: Seal,
    #[serde(default)]
    pub receipts: Vec<Receipt>,
}

/// Routing and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    /// Source DID (ASCII-only)
    pub src: String,
    /// Destination DID (ASCII-only, optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dst: Option<String>,
    /// 16-byte random nonce
    #[serde(with = "hex_bytes_16")]
    pub nonce: [u8; 16],
    /// Unix timestamp (milliseconds)
    pub ts: i64,
    /// Act: ATTEST | EVALUATE | TRANSACT
    pub act: String,
    /// Scope (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Expiration (epoch-nanos, optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
}

/// The payload envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub body: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub links: Option<Links>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

/// Links to prior capsules (for pipeline composition).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Links {
    /// CID of the prior capsule in the pipeline
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
}

/// Author seal: signature over {domain, id, hdr, env}.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seal {
    /// Key ID (ASCII-only DID#fragment)
    pub kid: String,
    /// Ed25519 signature (64 bytes)
    #[serde(with = "hex_bytes_64")]
    pub sig: [u8; 64],
    /// Scope tag for domain separation
    #[serde(default = "default_scope")]
    pub scope: String,
    /// Audience (must match hdr.dst if present)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
}

fn default_scope() -> String {
    "capsule".into()
}

/// A single receipt (SIRP hop) in the custody chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// blake3(nrf.encode(receipt_payload))
    #[serde(with = "hex_bytes_32")]
    pub id: [u8; 32],
    /// Capsule ID this receipt belongs to
    #[serde(with = "hex_bytes_32")]
    pub of: [u8; 32],
    /// Previous receipt ID (zeros for first hop)
    #[serde(with = "hex_bytes_32")]
    pub prev: [u8; 32],
    /// Kind: relay | exec | deliver
    pub kind: String,
    /// Node DID (ASCII-only)
    pub node: String,
    /// Unix timestamp (milliseconds)
    pub ts: i64,
    /// Ed25519 signature (64 bytes)
    #[serde(with = "hex_bytes_64")]
    pub sig: [u8; 64],
}

// ---------------------------------------------------------------------------
// Hex serde helpers for fixed-size byte arrays
// ---------------------------------------------------------------------------

mod hex_bytes_32 {
    use serde::{self, Deserialize, Deserializer, Serializer};
    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let v = hex::decode(&s).map_err(serde::de::Error::custom)?;
        let arr: [u8; 32] = v
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 32 bytes"))?;
        Ok(arr)
    }
}

mod hex_bytes_16 {
    use serde::{self, Deserialize, Deserializer, Serializer};
    pub fn serialize<S>(bytes: &[u8; 16], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 16], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let v = hex::decode(&s).map_err(serde::de::Error::custom)?;
        let arr: [u8; 16] = v
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 16 bytes"))?;
        Ok(arr)
    }
}

mod hex_bytes_64 {
    use serde::{self, Deserialize, Deserializer, Serializer};
    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let v = hex::decode(&s).map_err(serde::de::Error::custom)?;
        let arr: [u8; 64] = v
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 64 bytes"))?;
        Ok(arr)
    }
}
