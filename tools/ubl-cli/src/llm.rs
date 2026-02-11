use anyhow::{anyhow, Result};
use clap::{Args, ValueEnum};
use reqwest::Client;
use serde::Deserialize;

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum LlmProvider {
    Openai,
    Ollama,
    Registry,
}

#[derive(Args, Debug)]
pub struct CompleteArgs {
    /// Provider: openai | ollama | registry
    #[arg(long, value_enum, default_value = "registry")]
    pub provider: LlmProvider,
    /// Model name (gpt-4o-mini, llama3.1:8b, etc.)
    #[arg(long, default_value = "gpt-4o-mini")]
    pub model: String,
    /// Max tokens
    #[arg(long, default_value_t = 512)]
    pub max_tokens: u32,
    /// Path to prompt file (plain text or JSON with {"prompt": "..."})
    #[arg(long)]
    pub input: String,
    /// Force JSON mode if provider supports it
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct JudgeArgs {
    /// Answer file
    #[arg(long)]
    pub answer: String,
    /// Criteria text file (simple rubric, one criterion per line)
    #[arg(long)]
    pub criteria: String,
    /// Optional provider; if omitted, uses local heuristic
    #[arg(long, value_enum)]
    pub provider: Option<LlmProvider>,
    #[arg(long, default_value = "gpt-4o-mini")]
    pub model: String,
    #[arg(long, default_value_t = 256)]
    pub max_tokens: u32,
}

#[derive(Deserialize)]
struct SimplePrompt {
    prompt: String,
}

fn read_prompt(path: &str) -> Result<String> {
    let s = std::fs::read_to_string(path)?;
    if let Ok(sp) = serde_json::from_str::<SimplePrompt>(&s) {
        Ok(sp.prompt)
    } else {
        Ok(s)
    }
}

pub async fn llm_complete(args: CompleteArgs) -> Result<()> {
    let prompt = read_prompt(&args.input)?;
    match args.provider {
        LlmProvider::Registry => complete_via_registry(&prompt, &args.model, args.max_tokens, args.json).await,
        LlmProvider::Openai => complete_via_openai(&prompt, &args.model, args.max_tokens, args.json).await,
        LlmProvider::Ollama => complete_via_ollama(&prompt, &args.model, args.max_tokens).await,
    }
}

pub async fn llm_judge(args: JudgeArgs) -> Result<()> {
    let answer = std::fs::read_to_string(&args.answer)?;
    let criteria = std::fs::read_to_string(&args.criteria)?;
    if let Some(p) = args.provider {
        let prompt = format!(
            "JUDGE the following answer against the criteria.\n\nCRITERIA:\n{criteria}\n\nANSWER:\n{answer}\n\nRespond JSON {{\"score\":0..1,\"ok\":bool,\"feedback\":string}}."
        );
        match p {
            LlmProvider::Registry => complete_via_registry(&prompt, &args.model, args.max_tokens, true).await,
            LlmProvider::Openai => complete_via_openai(&prompt, &args.model, args.max_tokens, true).await,
            LlmProvider::Ollama => complete_via_ollama(&prompt, &args.model, args.max_tokens).await,
        }
    } else {
        let score = local_heuristic_score(&answer, &criteria);
        let ok = score >= 0.6;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "provider": "local",
                "score": score,
                "ok": ok,
            }))?
        );
        Ok(())
    }
}

fn local_heuristic_score(answer: &str, criteria: &str) -> f32 {
    let ans = answer.to_lowercase();
    let items: Vec<&str> = criteria
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() {
        return 0.0;
    }
    let hits = items
        .iter()
        .filter(|kw| ans.contains(&kw.to_lowercase()))
        .count();
    (hits as f32 / items.len() as f32).min(1.0)
}

// ---------------------------------------------------------------------------
// Provider: Registry
// ---------------------------------------------------------------------------

fn registry_base() -> String {
    std::env::var("REGISTRY_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8791".into())
}

#[cfg(feature = "provider-registry")]
async fn complete_via_registry(prompt: &str, model: &str, max_tokens: u32, json_mode: bool) -> Result<()> {
    let url = format!("{}/v1/llm/complete", registry_base());
    let body = serde_json::json!({
        "prompt": prompt,
        "model": model,
        "max_tokens": max_tokens,
        "json_mode": json_mode,
    });
    let resp = Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    let v: serde_json::Value = resp.json().await?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}

#[cfg(not(feature = "provider-registry"))]
async fn complete_via_registry(_: &str, _: &str, _: u32, _: bool) -> Result<()> {
    Err(anyhow!("compiled without feature provider-registry"))
}

// ---------------------------------------------------------------------------
// Provider: OpenAI
// ---------------------------------------------------------------------------

#[cfg(feature = "provider-openai")]
async fn complete_via_openai(prompt: &str, model: &str, max_tokens: u32, json_mode: bool) -> Result<()> {
    let key = std::env::var("OPENAI_API_KEY").map_err(|_| anyhow!("OPENAI_API_KEY not set"))?;
    let mut body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
    });
    if json_mode {
        body["response_format"] = serde_json::json!({"type": "json_object"});
    }
    let resp = Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    let v: serde_json::Value = resp.json().await?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}

#[cfg(not(feature = "provider-openai"))]
async fn complete_via_openai(_: &str, _: &str, _: u32, _: bool) -> Result<()> {
    Err(anyhow!("compiled without feature provider-openai"))
}

// ---------------------------------------------------------------------------
// Provider: Ollama
// ---------------------------------------------------------------------------

#[cfg(feature = "provider-ollama")]
async fn complete_via_ollama(prompt: &str, model: &str, max_tokens: u32) -> Result<()> {
    let base = std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".into());
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "options": { "num_predict": max_tokens },
    });
    let resp = Client::new()
        .post(format!("{base}/api/chat"))
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    let v: serde_json::Value = resp.json().await?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}

#[cfg(not(feature = "provider-ollama"))]
async fn complete_via_ollama(_: &str, _: &str, _: u32) -> Result<()> {
    Err(anyhow!("compiled without feature provider-ollama"))
}
