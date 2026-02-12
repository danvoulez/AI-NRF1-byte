//! Capability registry: register and lookup capabilities by kind + version.

use std::sync::Arc;
use modules_core::Capability;

pub struct CapRegistry {
    inner: Vec<Arc<dyn Capability>>,
}

impl CapRegistry {
    pub fn new() -> Self {
        Self { inner: vec![] }
    }

    pub fn register<C: Capability + 'static>(&mut self, c: C) {
        self.inner.push(Arc::new(c));
    }

    pub fn get(&self, kind: &str, version_req: &str) -> Option<Arc<dyn Capability>> {
        self.inner
            .iter()
            .find(|c| c.kind() == kind && semver_match(c.api_version(), version_req))
            .cloned()
    }
}

impl Default for CapRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn semver_match(actual: &str, req: &str) -> bool {
    if req == "*" {
        return true;
    }
    let req = req.strip_prefix('^').unwrap_or(req);
    actual.split('.').next() == req.split('.').next()
}
