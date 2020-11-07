use crate::core::identifiers::Identifier;

pub trait Command {
    fn command(&self) -> &str;
    fn dependencies(&self) -> Vec<Identifier> {
        Identifier::parse(self.command())
    }
}
