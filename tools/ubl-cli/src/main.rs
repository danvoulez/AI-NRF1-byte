//! `ubl` CLI — operate UBL Capsules end-to-end.
//!
//! Usage:
//!   ubl cap from-json  <in.json>  -o <out.nrf|->
//!   ubl cap to-json    <in.nrf>   -o <out.json|->
//!   ubl cap hash       <in.(json|nrf)>
//!   ubl cap sign       <in.json>  --sk <file>
//!   ubl cap verify     <in.(json|nrf)> --pk <file>
//!   ubl cap receipt add <in> --kind <relay|exec|deliver> --node <did#key> --sk <file> -o <out>
//!   ubl keygen          -o <prefix>

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::{collections::HashMap, path::Path};

#[derive(Parser)]
#[command(name = "ubl", version, about = "UBL Capsule CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Capsule operations
    Cap {
        #[command(subcommand)]
        action: CapAction,
    },
    /// Generate an Ed25519 keypair
    Keygen {
        /// Output prefix (creates <prefix>.sk and <prefix>.pk)
        #[arg(short, long, default_value = "key")]
        output: String,
    },
}

#[derive(Subcommand)]
enum CapAction {
    /// JSON → NRF binary
    FromJson {
        /// Input JSON file (or - for stdin)
        input: String,
        /// Output NRF file (or - for stdout)
        #[arg(short, long, default_value = "-")]
        output: String,
    },
    /// NRF binary → JSON
    ToJson {
        /// Input NRF file (or - for stdin)
        input: String,
        /// Output JSON file (or - for stdout)
        #[arg(short, long, default_value = "-")]
        output: String,
    },
    /// Compute blake3 hash of canonical NRF encoding
    Hash {
        /// Input file (JSON or NRF, or - for stdin)
        input: String,
    },
    /// Sign a capsule JSON
    Sign {
        /// Input capsule JSON file
        input: String,
        /// Ed25519 secret key file (32 bytes hex)
        #[arg(long)]
        sk: PathBuf,
        /// Output signed capsule JSON (or - for stdout)
        #[arg(short, long, default_value = "-")]
        output: String,
    },
    /// Verify a capsule's seal
    Verify {
        /// Input capsule (JSON or NRF)
        input: String,
        /// Ed25519 public key file (32 bytes hex)
        #[arg(long)]
        pk: PathBuf,
        /// Allowed clock skew for expiry checks (nanoseconds)
        #[arg(long, default_value_t = 0)]
        allowed_skew_ns: i64,
        /// Also verify the receipts chain (requires `--keyring`)
        #[arg(long)]
        verify_chain: bool,
        /// JSON file mapping node DID#key -> hex-encoded public key (32 bytes)
        #[arg(long)]
        keyring: Option<PathBuf>,
    },
    /// Receipt operations
    Receipt {
        #[command(subcommand)]
        action: ReceiptAction,
    },
}

#[derive(Subcommand)]
enum ReceiptAction {
    /// Add a receipt hop to a capsule
    Add {
        /// Input capsule JSON file
        input: String,
        /// Hop kind: relay | exec | deliver
        #[arg(long)]
        kind: String,
        /// Node DID#key (ASCII-only)
        #[arg(long)]
        node: String,
        /// Ed25519 secret key file (32 bytes hex)
        #[arg(long)]
        sk: PathBuf,
        /// Output capsule JSON (or - for stdout)
        #[arg(short, long, default_value = "-")]
        output: String,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Cap { action } => match action {
            CapAction::FromJson { input, output } => cmd_from_json(&input, &output),
            CapAction::ToJson { input, output } => cmd_to_json(&input, &output),
            CapAction::Hash { input } => cmd_hash(&input),
            CapAction::Sign { input, sk, output } => cmd_sign(&input, &sk, &output),
            CapAction::Verify {
                input,
                pk,
                allowed_skew_ns,
                verify_chain,
                keyring,
            } => cmd_verify(
                &input,
                &pk,
                allowed_skew_ns,
                verify_chain,
                keyring.as_deref(),
            ),
            CapAction::Receipt { action } => match action {
                ReceiptAction::Add {
                    input,
                    kind,
                    node,
                    sk,
                    output,
                } => cmd_receipt_add(&input, &kind, &node, &sk, &output),
            },
        },
        Commands::Keygen { output } => cmd_keygen(&output),
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn cmd_from_json(input: &str, output: &str) -> Result<()> {
    let json_str = read_input(input)?;
    let j: serde_json::Value = serde_json::from_str(&json_str).context("Err.Parse.InvalidJSON")?;
    let nrf_bytes = ubl_json_view::json_to_nrf_bytes(&j).map_err(|e| anyhow!("Err.Canon.{e}"))?;
    write_output(output, &nrf_bytes)?;
    Ok(())
}

fn cmd_to_json(input: &str, output: &str) -> Result<()> {
    let nrf_bytes = read_input_bytes(input)?;
    let j = ubl_json_view::nrf_bytes_to_json(&nrf_bytes).map_err(|e| anyhow!("Err.Decode.{e}"))?;
    let json_str = serde_json::to_string_pretty(&j)?;
    write_output(output, json_str.as_bytes())?;
    Ok(())
}

fn cmd_hash(input: &str) -> Result<()> {
    let data = read_input_bytes(input)?;
    // Try NRF first, fall back to JSON
    let nrf_bytes = if data.starts_with(b"nrf1") {
        data
    } else {
        let json_str = std::str::from_utf8(&data).context("Err.Parse.InvalidUTF8")?;
        let j: serde_json::Value =
            serde_json::from_str(json_str).context("Err.Parse.InvalidJSON")?;
        ubl_json_view::json_to_nrf_bytes(&j).map_err(|e| anyhow!("Err.Canon.{e}"))?
    };
    let hash = blake3::hash(&nrf_bytes);
    println!("b3:{}", hash.to_hex());
    Ok(())
}

fn cmd_sign(input: &str, sk_path: &PathBuf, output: &str) -> Result<()> {
    let json_str = read_input(input)?;
    let mut capsule: ubl_capsule::Capsule =
        serde_json::from_str(&json_str).context("Err.Parse.InvalidCapsuleJSON")?;
    let sk = load_signing_key(sk_path)?;
    ubl_capsule::seal::sign(&mut capsule, &sk);
    let out = serde_json::to_string_pretty(&capsule)?;
    write_output(output, out.as_bytes())?;
    Ok(())
}

fn cmd_verify(
    input: &str,
    pk_path: &PathBuf,
    allowed_skew_ns: i64,
    verify_chain: bool,
    keyring_path: Option<&Path>,
) -> Result<()> {
    let json_str = read_input(input)?;
    let capsule: ubl_capsule::Capsule =
        serde_json::from_str(&json_str).context("Err.Parse.InvalidCapsuleJSON")?;
    let pk = load_verifying_key(pk_path)?;
    let opts = ubl_capsule::seal::VerifyOpts { allowed_skew_ns };
    ubl_capsule::seal::verify_with_opts(&capsule, &pk, &opts).map_err(|e| anyhow!("{e}"))?;

    if verify_chain {
        let keyring_path =
            keyring_path.ok_or_else(|| anyhow!("Err.Args.MissingKeyring: --keyring required"))?;
        let pks = load_keyring(keyring_path)?;
        let resolve =
            |node: &str| -> Option<ed25519_dalek::VerifyingKey> { pks.get(node).copied() };
        ubl_capsule::receipt::verify_chain(&capsule.id, &capsule.receipts, &resolve)
            .map_err(|e| anyhow!("{e}"))?;
        println!("OK: seal + receipts chain verified");
    } else {
        println!("OK: seal verified");
    }
    Ok(())
}

fn cmd_receipt_add(
    input: &str,
    kind: &str,
    node: &str,
    sk_path: &PathBuf,
    output: &str,
) -> Result<()> {
    let json_str = read_input(input)?;
    let mut capsule: ubl_capsule::Capsule =
        serde_json::from_str(&json_str).context("Err.Parse.InvalidCapsuleJSON")?;

    let sk = load_signing_key(sk_path)?;

    // Determine prev: last receipt's id, or [0u8; 32] for first hop
    let prev = capsule.receipts.last().map(|r| r.id).unwrap_or([0u8; 32]);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as i64;

    let receipt = ubl_capsule::receipt::add_hop(capsule.id, prev, kind, node, ts, &sk)
        .map_err(|e| anyhow!("{e}"))?;
    capsule.receipts.push(receipt);

    let out = serde_json::to_string_pretty(&capsule)?;
    write_output(output, out.as_bytes())?;
    Ok(())
}

fn cmd_keygen(prefix: &str) -> Result<()> {
    let sk = ed25519_dalek::SigningKey::generate(&mut rand_core::OsRng);
    let vk = sk.verifying_key();
    let sk_hex = hex::encode(sk.to_bytes());
    let pk_hex = hex::encode(vk.to_bytes());

    let sk_path = format!("{prefix}.sk");
    let pk_path = format!("{prefix}.pk");

    std::fs::write(&sk_path, &sk_hex).context("writing secret key")?;
    std::fs::write(&pk_path, &pk_hex).context("writing public key")?;

    println!("Secret key: {sk_path}");
    println!("Public key: {pk_path}");
    Ok(())
}

fn load_keyring(path: &Path) -> Result<HashMap<String, ed25519_dalek::VerifyingKey>> {
    let s = std::fs::read_to_string(path)
        .with_context(|| format!("reading keyring {}", path.display()))?;
    let m: HashMap<String, String> =
        serde_json::from_str(&s).context("Err.Parse.InvalidKeyringJSON")?;
    let mut out = HashMap::new();
    for (node, pk_hex) in m {
        let bytes = hex::decode(pk_hex.trim()).context("Err.Key.BadHex")?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow!("Err.Key.BadLength: expected 32 bytes (64 hex chars)"))?;
        let pk =
            ed25519_dalek::VerifyingKey::from_bytes(&arr).context("Err.Key.InvalidPublicKey")?;
        out.insert(node, pk);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// I/O helpers
// ---------------------------------------------------------------------------

fn read_input(path: &str) -> Result<String> {
    if path == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path).with_context(|| format!("reading {path}"))
    }
}

fn read_input_bytes(path: &str) -> Result<Vec<u8>> {
    if path == "-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        std::fs::read(path).with_context(|| format!("reading {path}"))
    }
}

fn write_output(path: &str, data: &[u8]) -> Result<()> {
    if path == "-" {
        std::io::stdout().write_all(data)?;
        std::io::stdout().flush()?;
    } else {
        std::fs::write(path, data).with_context(|| format!("writing {path}"))?;
    }
    Ok(())
}

fn load_signing_key(path: &PathBuf) -> Result<ed25519_dalek::SigningKey> {
    let hex_str = std::fs::read_to_string(path)
        .with_context(|| format!("reading secret key from {}", path.display()))?;
    let bytes = hex::decode(hex_str.trim()).context("Err.Key.BadHex")?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("Err.Key.BadLength: expected 32 bytes (64 hex chars)"))?;
    Ok(ed25519_dalek::SigningKey::from_bytes(&arr))
}

fn load_verifying_key(path: &PathBuf) -> Result<ed25519_dalek::VerifyingKey> {
    let hex_str = std::fs::read_to_string(path)
        .with_context(|| format!("reading public key from {}", path.display()))?;
    let bytes = hex::decode(hex_str.trim()).context("Err.Key.BadHex")?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("Err.Key.BadLength: expected 32 bytes (64 hex chars)"))?;
    ed25519_dalek::VerifyingKey::from_bytes(&arr).context("Err.Key.InvalidPublicKey")
}
