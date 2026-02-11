use anyhow::{anyhow, Result};
use clap::Args;
use reqwest::Client;
use serde_json::Value;

fn registry_base() -> String {
    std::env::var("REGISTRY_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8791".into())
}

fn read_json(path_or_inline: &str) -> Result<Value> {
    if std::path::Path::new(path_or_inline).exists() {
        Ok(serde_json::from_str(&std::fs::read_to_string(
            path_or_inline,
        )?)?)
    } else {
        Ok(serde_json::from_str(path_or_inline)?)
    }
}

// ---------------------------------------------------------------------------
// ubl pricing price — single SKU pricing
// ---------------------------------------------------------------------------

#[derive(Args, Debug)]
pub struct PriceArgs {
    /// JSON input: {"sku":"...", "qty":N, ...} or path to file
    #[arg(long)]
    pub input: String,
}

pub async fn price(args: PriceArgs) -> Result<()> {
    let payload = read_json(&args.input)?;
    let url = format!("{}/v1/pricing/price", registry_base());
    let resp = Client::new()
        .post(&url)
        .json(&payload)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| anyhow!("registry: {e}"))?;
    let v: Value = resp.json().await?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}

// ---------------------------------------------------------------------------
// ubl pricing quote — create a quote from pricing
// ---------------------------------------------------------------------------

#[derive(Args, Debug)]
pub struct QuoteArgs {
    /// JSON input with items/sku/quantity, or path to file
    #[arg(long)]
    pub input: String,
}

pub async fn quote(args: QuoteArgs) -> Result<()> {
    let payload = read_json(&args.input)?;
    let url = format!("{}/v1/quote/create", registry_base());
    let resp = Client::new()
        .post(&url)
        .json(&payload)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| anyhow!("registry: {e}"))?;
    let v: Value = resp.json().await?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}

// ---------------------------------------------------------------------------
// ubl pricing invoice — create invoice from quote
// ---------------------------------------------------------------------------

#[derive(Args, Debug)]
pub struct InvoiceArgs {
    /// JSON with quote_id + customer info, or path to file
    #[arg(long)]
    pub input: String,
}

pub async fn invoice(args: InvoiceArgs) -> Result<()> {
    let payload = read_json(&args.input)?;
    let url = format!("{}/v1/invoice/create_from_quote", registry_base());
    let resp = Client::new()
        .post(&url)
        .json(&payload)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| anyhow!("registry: {e}"))?;
    let v: Value = resp.json().await?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}
