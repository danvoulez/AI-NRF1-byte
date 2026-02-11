use std::sync::Arc;
use modules_core::Capability;

pub struct CapRegistry {
    inner: Vec<Arc<dyn Capability>>,
}
impl CapRegistry {
    pub fn new() -> Self { Self { inner: vec![] } }
    pub fn register<C: Capability + 'static>(&mut self, c: C) { self.inner.push(Arc::new(c)); }

    pub fn get(&self, kind: &str, version_req: &str) -> Option<Arc<dyn Capability>> {
        self.inner.iter().find(|c| {
            c.as_ref().kind() == kind && semver_match(c.as_ref().api_version(), version_req)
        }).cloned()
    }
}
trait CapIntrospect {
    fn kind(&self) -> &str;
    fn api_version(&self) -> &str;
}
impl<T: Capability> CapIntrospect for T {
    fn kind(&self) -> &str { T::KIND }
    fn api_version(&self) -> &str { T::API_VERSION }
}
fn semver_match(actual: &str, req: &str) -> bool {
    let req = req.strip_prefix('^').unwrap_or(req);
    actual.split('.').next() == req.split('.').next() // major compat simples
}
