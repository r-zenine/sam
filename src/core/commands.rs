use crate::core::identifiers::Identifier;
use crate::core::namespaces::Namespace;

pub trait Command: Namespace {
    // Returns a string representation of a command
    fn command(&self) -> &str;
    // Returns the dependencies of an command.
    fn dependencies(&self) -> Vec<Identifier> {
        Identifier::parse(self.command(), self.namespace())
    }
}
