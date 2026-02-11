use modules_core::{Capability, CapInput, CapOutput, ExecutionMeta, Verdict};
use crate::{cap_registry::CapRegistry, effects::EffectExecutor};
use blake3;
use time::OffsetDateTime;

pub struct Runner<'a, E: EffectExecutor> {
    pub caps: &'a CapRegistry,
    pub assets: Box<dyn modules_core::AssetResolver>,
    pub effects: &'a E,
}
impl<'a, E: EffectExecutor> Runner<'a, E> {
    pub fn new(caps: &'a CapRegistry, assets: Box<dyn modules_core::AssetResolver>, effects: &'a E) -> Self {
        Self { caps, assets, effects }
    }

    pub async fn run(
        &self,
        manifest: &crate::manifest::Manifest,
        mut env: ai_nrf1::Value
    ) -> anyhow::Result<(ai_nrf1::Value, Vec<[u8;32]>, Verdict)> {

        let mut receipts: Vec<[u8;32]> = vec![];
        let mut verdict_final = Verdict::Allow;

        for step in &manifest.pipeline {
            let cap = self.caps
                .get(&step.kind, &step.version)
                .ok_or_else(|| anyhow::anyhow!(format!("cap not found: {} {}", step.kind, step.version)))?;

            cap.validate_config(&step.config)?;

            let input = CapInput {
                env: env.clone(),
                config: step.config.clone(),
                assets: self.assets.box_clone(),
                prev_receipts: receipts.clone(),
                meta: ExecutionMeta {
                    run_id: format!("run-{}", now_nanos()),
                    tenant: None,
                    trace_id: None,
                    ts_nanos: now_nanos(),
                },
            };

            let out: CapOutput = cap.execute(input)?;
            if let Some(new_env) = out.new_env { env = new_env; }
            if let Some(v) = out.verdict.clone() { verdict_final = v; }

            // hop-id: hash do (step_id + kind + versão + resumo do output)
            let hop_id = hop_payload_id(step, &out);
            receipts.push(hop_id);

            for eff in &out.effects {
                self.effects.execute(eff).await?;
            }

            match verdict_final {
                Verdict::Deny => break,
                _ => {}
            }
        }

        Ok((env, receipts, verdict_final))
    }
}

fn now_nanos() -> i64 {
    let t = OffsetDateTime::now_utc();
    (t.unix_timestamp_nanos() as i128) as i64
}

fn hop_payload_id(step: &crate::manifest::Step, out: &CapOutput) -> [u8;32] {
    // determinístico e simples (não é o receipt final assinado; é o "id do hop payload")
    let mut hasher = blake3::Hasher::new();
    hasher.update(step.step_id.as_bytes());
    hasher.update(step.kind.as_bytes());
    hasher.update(step.version.as_bytes());
    if let Some(v) = &out.verdict { hasher.update(format!("{v:?}").as_bytes()); }
    hasher.update(&(out.metrics.len() as u64).to_le_bytes());
    *hasher.finalize().as_bytes()
}
