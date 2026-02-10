use clap::{Parser, Subcommand};
use nrf_core::{decode, encode, hash_bytes};
use serde_json::Value as J;
use std::io::{self, Read, Write};
/// ai-nrf1 (UBL-Byte) CLI: encode/decode/hash
#[derive(Parser)]
#[command(
    name = "nrf1",
    version,
    about = "ai-nrf1 (UBL-Byte) CLI — encode/decode/hash over canonical bytes"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Encode JSON -> ai-nrf1 bytes
    Encode {
        /// Input JSON file (or - for stdin)
        input: String,
        /// Output file (or - for stdout). Writes raw bytes.
        #[arg(short, long)]
        out: Option<String>,
    },
    /// Decode ai-nrf1 bytes -> JSON (view only — hash is always over bytes)
    Decode {
        /// Input NRF file (or - for stdin)
        input: String,
        /// Output file (or - for stdout). Writes pretty-printed JSON.
        #[arg(short, long)]
        out: Option<String>,
    },
    /// Compute BLAKE3 of raw bytes (NRF or any file)
    Hash {
        /// Input file (or - for stdin)
        input: String,
        /// print as 'b3:<hex>'
        #[arg(short, long)]
        tag: bool,
    },
}

// JSON <-> NRF mapping (canonical, zero-choice)

fn parse_hex_lower(s: &str) -> anyhow::Result<Vec<u8>> {
    nrf_core::parse_hex_lower(s).map_err(|e| anyhow::anyhow!("{e}"))
}

fn to_hex_lower(bytes: &[u8]) -> String {
    nrf_core::encode_hex_lower(bytes)
}

fn json_to_nrf(v: &J) -> anyhow::Result<nrf_core::Value> {
    use nrf_core::Value as V;
    Ok(match v {
        J::Null => V::Null,
        J::Bool(b) => V::Bool(*b),
        J::Number(n) => {
            if !n.is_i64() {
                anyhow::bail!("floats not allowed; numbers must be i64");
            }
            V::Int(n.as_i64().unwrap())
        }
        J::String(s) => {
            nrf_core::validate_nfc(s).map_err(|e| anyhow::anyhow!("{e}"))?;
            V::String(s.clone())
        }
        J::Array(arr) => V::Array(
            arr.iter()
                .map(json_to_nrf)
                .collect::<anyhow::Result<Vec<_>>>()?,
        ),
        J::Object(map) => {
            if map.len() == 1 {
                if let Some(J::String(hex)) = map.get("$bytes") {
                    return Ok(V::Bytes(parse_hex_lower(hex)?));
                }
            }
            let mut out = std::collections::BTreeMap::new();
            for (k, val) in map {
                out.insert(k.clone(), json_to_nrf(val)?);
            }
            V::Map(out)
        }
    })
}

fn nrf_to_json(v: &nrf_core::Value) -> J {
    use nrf_core::Value as V;
    match v {
        V::Null => J::Null,
        V::Bool(b) => J::Bool(*b),
        V::Int(i) => serde_json::Number::from(*i).into(),
        V::String(s) => J::String(s.clone()),
        V::Bytes(b) => {
            let mut o = serde_json::Map::new();
            o.insert("$bytes".to_string(), J::String(to_hex_lower(b)));
            J::Object(o)
        }
        V::Array(items) => J::Array(items.iter().map(nrf_to_json).collect()),
        V::Map(m) => {
            let mut o = serde_json::Map::new();
            for (k, val) in m {
                o.insert(k.clone(), nrf_to_json(val));
            }
            J::Object(o)
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Encode { input, out } => {
            let j = read_to_string_maybe_stdin(&input)?;
            let vj: J = serde_json::from_str(&j)?;
            let v = json_to_nrf(&vj)?;
            let bytes = encode(&v);
            write_bytes_maybe_stdout(out.as_deref(), &bytes)?;
        }
        Cmd::Decode { input, out } => {
            let bytes = read_to_bytes_maybe_stdin(&input)?;
            let v = decode(&bytes).map_err(|e| anyhow::anyhow!(format!("{e:?}")))?;
            let j = nrf_to_json(&v);
            let s = serde_json::to_string_pretty(&j)? + "\n";
            write_string_maybe_stdout(out.as_deref(), &s)?;
        }
        Cmd::Hash { input, tag } => {
            let bytes = read_to_bytes_maybe_stdin(&input)?;
            let h = hash_bytes(&bytes);
            let hex = hex::encode(h);
            if tag {
                println!("b3:{hex}");
            } else {
                println!("{hex}");
            }
        }
    }
    Ok(())
}

fn read_to_string_maybe_stdin(path: &str) -> anyhow::Result<String> {
    if path == "-" {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s)?;
        Ok(s)
    } else {
        Ok(std::fs::read_to_string(path)?)
    }
}

fn read_to_bytes_maybe_stdin(path: &str) -> anyhow::Result<Vec<u8>> {
    if path == "-" {
        let mut v = Vec::new();
        io::stdin().read_to_end(&mut v)?;
        Ok(v)
    } else {
        Ok(std::fs::read(path)?)
    }
}

fn write_bytes_maybe_stdout(path: Option<&str>, bytes: &[u8]) -> anyhow::Result<()> {
    match path {
        Some("-") | None => {
            let mut stdout = io::stdout();
            stdout.write_all(bytes)?;
        }
        Some(p) => {
            std::fs::write(p, bytes)?;
        }
    }
    Ok(())
}

fn write_string_maybe_stdout(path: Option<&str>, s: &str) -> anyhow::Result<()> {
    match path {
        Some("-") | None => {
            print!("{s}");
        }
        Some(p) => {
            std::fs::write(p, s)?;
        }
    }
    Ok(())
}
