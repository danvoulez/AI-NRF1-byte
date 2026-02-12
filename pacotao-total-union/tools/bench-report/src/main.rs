use clap::Parser;
use anyhow::Result;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Parser)]
struct Cli { #[arg(long)] inputs: Vec<String> }

#[derive(Deserialize)]
struct Row { latency_ms: Option<f64>, cost_usd: Option<f64>, quality_score: Option<f64> }

fn main() -> Result<()> {
    let cli = Cli::parse();
    println!("# Bench Report\n");
    for path in &cli.inputs {
        println!("## {}", path);
        let f = File::open(path)?;
        let rdr = BufReader::new(f);
        let mut n=0usize; let mut lat=0.0; let mut cost=0.0; let mut qual=0.0;
        for line in rdr.lines() {
            let l = line?; if l.trim().is_empty() { continue; }
            if let Ok(row) = serde_json::from_str::<Row>(&l) {
                n+=1; lat+=row.latency_ms.unwrap_or(0.0);
                cost+=row.cost_usd.unwrap_or(0.0);
                qual+=row.quality_score.unwrap_or(0.0);
            }
        }
        if n>0 {
            println!("- runs: {n}");
            println!("- avg latency: {:.2} ms", lat/n as f64);
            println!("- avg cost: ${:.5}", cost/n as f64);
            println!("- avg quality: {:.3}", qual/n as f64);
            println!();
        }
    }
    Ok(())
}
