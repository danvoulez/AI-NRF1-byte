//! In-memory AssetResolver for tests and development.

use std::collections::HashMap;
use std::sync::Arc;
use modules_core::{Asset, AssetResolver, Cid};

#[derive(Clone)]
pub struct MemoryResolver {
    items: Arc<HashMap<Cid, Asset>>,
}

impl MemoryResolver {
    pub fn new() -> Self {
        Self {
            items: Arc::new(HashMap::new()),
        }
    }

    pub fn with(mut self, a: Asset) -> Self {
        let mut m = (*self.items).clone();
        m.insert(a.cid, a);
        self.items = Arc::new(m);
        self
    }
}

impl Default for MemoryResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetResolver for MemoryResolver {
    fn get(&self, cid: &Cid) -> anyhow::Result<Asset> {
        self.items
            .get(cid)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("asset not found"))
    }

    fn box_clone(&self) -> Box<dyn AssetResolver> {
        Box::new(self.clone())
    }
}
