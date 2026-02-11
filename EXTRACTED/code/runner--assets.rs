use modules_core::{AssetResolver, Asset, Cid};

#[derive(Clone)]
pub struct MemoryResolver {
    items: std::sync::Arc<std::collections::HashMap<Cid, Asset>>,
}
impl MemoryResolver {
    pub fn new() -> Self {
        Self { items: std::sync::Arc::new(std::collections::HashMap::new()) }
    }
    pub fn with(mut self, a: Asset) -> Self {
        let mut m = (*self.items).clone();
        m.insert(a.cid, a);
        self.items = std::sync::Arc::new(m);
        self
    }
}
#[async_trait::async_trait]
impl AssetResolver for MemoryResolver {
    fn get(&self, cid: &Cid) -> anyhow::Result<Asset> {
        self.items.get(cid).cloned().ok_or_else(|| anyhow::anyhow!("asset not found"))
    }
    fn box_clone(&self) -> Box<dyn AssetResolver> {
        Box::new(self.clone())
    }
}
