//! Pipeline runner: iterates manifest steps, calls capabilities, collects hop receipts.
//! See design doc §10: "Runtime/Orquestrador".
//!
//! Flow control:
//!   - `ALLOW` → continue to next step
//!   - `DENY`  → break, finalize with DENY
//!   - `REQUIRE` → execute effects (QueueConsentTicket), break with REQUIRE (pending)

use modules_core::{CapInput, CapOutput, ExecutionMeta, Verdict};
use crate::cap_registry::CapRegistry;
use crate::effects::{EffectExecutor, ExecCtx};
use crate::manifest::Manifest;

/// Result of a pipeline run.
#[derive(Debug)]
pub struct RunResult {
    pub env: nrf1::Value,
    pub receipts: Vec<[u8; 32]>,
    pub verdict: Verdict,
    /// Step ID where the pipeline stopped (None = completed all steps).
    pub stopped_at: Option<String>,
    /// All artifacts collected across steps.
    pub artifacts: Vec<modules_core::Artifact>,
    /// Per-step metrics: (step_id, key, value).
    pub step_metrics: Vec<(String, String, i64)>,
}

pub struct Runner<'a, E: EffectExecutor> {
    pub caps: &'a CapRegistry,
    pub assets: Box<dyn modules_core::AssetResolver>,
    pub effects: &'a E,
    pub io_bindings: serde_json::Value,
    pub tenant: String,
}

impl<'a, E: EffectExecutor> Runner<'a, E> {
    pub fn new(
        caps: &'a CapRegistry,
        assets: Box<dyn modules_core::AssetResolver>,
        effects: &'a E,
        io_bindings: serde_json::Value,
        tenant: impl Into<String>,
    ) -> Self {
        Self {
            caps,
            assets,
            effects,
            io_bindings,
            tenant: tenant.into(),
        }
    }

    pub async fn run(
        &self,
        manifest: &Manifest,
        mut env: nrf1::Value,
    ) -> anyhow::Result<RunResult> {
        let run_id = format!("run-{}", now_nanos());
        let trace_id = format!("{:016x}", now_nanos());
        let capsule_id_hex = {
            let env_bytes = nrf1::encode(&env);
            hex::encode(blake3::hash(&env_bytes).as_bytes())
        };

        let mut receipts: Vec<[u8; 32]> = vec![];
        let mut verdict_final = Verdict::Allow;
        let mut stopped_at: Option<String> = None;
        let mut all_artifacts = vec![];
        let mut all_metrics = vec![];

        tracing::info!(
            run_id = %run_id,
            product = %manifest.name,
            tenant = %self.tenant,
            steps = manifest.pipeline.len(),
            "pipeline.start"
        );

        for step in &manifest.pipeline {
            let t0 = std::time::Instant::now();

            let cap = self
                .caps
                .get(&step.kind, &step.version)
                .ok_or_else(|| {
                    anyhow::anyhow!("cap not found: {} {}", step.kind, step.version)
                })?;

            cap.validate_config(&step.config)?;

            let ts = now_nanos();
            let input = CapInput {
                env: env.clone(),
                config: step.config.clone(),
                assets: self.assets.box_clone(),
                prev_receipts: receipts.clone(),
                meta: ExecutionMeta {
                    run_id: run_id.clone(),
                    tenant: Some(self.tenant.clone()),
                    trace_id: Some(trace_id.clone()),
                    ts_nanos: ts,
                },
            };

            let out: CapOutput = cap.execute(input)?;
            let elapsed_ms = t0.elapsed().as_millis() as i64;

            // Collect env update
            if let Some(ref new_env) = out.new_env {
                env = new_env.clone();
            }

            // Collect verdict
            if let Some(ref v) = out.verdict {
                verdict_final = v.clone();
            }

            // Collect artifacts
            all_artifacts.extend(out.artifacts.clone());

            // Collect metrics
            for (k, v) in &out.metrics {
                all_metrics.push((step.step_id.clone(), k.clone(), *v));
            }
            all_metrics.push((step.step_id.clone(), "duration_ms".into(), elapsed_ms));

            // Generate hop receipt
            let hop_id = hop_payload_id(step, &out);
            receipts.push(hop_id);

            // Build ExecCtx for effect dispatch
            let exec_ctx = ExecCtx {
                tenant: self.tenant.clone(),
                trace_id: trace_id.clone(),
                io_bindings: self.io_bindings.clone(),
                now_nanos: ts,
                step_id: step.step_id.clone(),
                capsule_id_hex: capsule_id_hex.clone(),
            };

            // Execute effects
            for eff in &out.effects {
                self.effects.execute(eff, &exec_ctx).await?;
            }

            tracing::info!(
                run_id = %run_id,
                step_id = %step.step_id,
                kind = %step.kind,
                verdict = ?verdict_final,
                effects = out.effects.len(),
                artifacts = out.artifacts.len(),
                elapsed_ms = elapsed_ms,
                "pipeline.step.done"
            );

            // Flow control
            match verdict_final {
                Verdict::Deny => {
                    stopped_at = Some(step.step_id.clone());
                    tracing::warn!(
                        run_id = %run_id,
                        step_id = %step.step_id,
                        "pipeline.halt (DENY)"
                    );
                    break;
                }
                Verdict::Require => {
                    stopped_at = Some(step.step_id.clone());
                    tracing::info!(
                        run_id = %run_id,
                        step_id = %step.step_id,
                        "pipeline.pending (REQUIRE — consent needed)"
                    );
                    break;
                }
                Verdict::Allow => {}
            }
        }

        tracing::info!(
            run_id = %run_id,
            verdict = ?verdict_final,
            receipts = receipts.len(),
            artifacts = all_artifacts.len(),
            "pipeline.end"
        );

        Ok(RunResult {
            env,
            receipts,
            verdict: verdict_final,
            stopped_at,
            artifacts: all_artifacts,
            step_metrics: all_metrics,
        })
    }
}

fn now_nanos() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as i64
}

fn hop_payload_id(step: &crate::manifest::Step, out: &CapOutput) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(step.step_id.as_bytes());
    hasher.update(step.kind.as_bytes());
    hasher.update(step.version.as_bytes());
    if let Some(v) = &out.verdict {
        hasher.update(format!("{v:?}").as_bytes());
    }
    hasher.update(&(out.metrics.len() as u64).to_le_bytes());
    *hasher.finalize().as_bytes()
}
