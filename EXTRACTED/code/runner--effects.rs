use async_trait::async_trait;
use modules_core::Effect;

#[async_trait]
pub trait EffectExecutor: Send + Sync {
    async fn execute(&self, effect: &Effect) -> anyhow::Result<()>;
}

pub struct NoopExecutor;
#[async_trait]
impl EffectExecutor for NoopExecutor {
    async fn execute(&self, _effect: &Effect) -> anyhow::Result<()> { Ok(()) }
}
