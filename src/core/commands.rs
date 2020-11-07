use crate::core::identifiers::Identifier;
use crate::core::namespaces::Namespace;

pub trait Command: Namespace {
    fn command(&self) -> &str;
    fn dependencies(&self) -> Vec<Identifier> {
        Identifier::parse(self.command(), self.namespace())
    }
}
