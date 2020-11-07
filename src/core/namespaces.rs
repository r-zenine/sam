use std::path::Path;
pub trait Namespace {
    fn namespace(&self) -> Option<&str>;
}

pub trait NamespaceUpdater {
    fn update(&mut self, namespace: impl Into<String>);

    fn update_from_path(&mut self, path: &Path) -> Option<()> {
        let namespace = path.parent().and_then(|e| e.as_os_str().to_str());
        namespace.map(|ns| self.update(ns))
    }
}
