use crate::core::commands::Command;
use crate::core::dependencies::Dependencies;
use crate::core::identifiers::Identifier;
use crate::core::namespaces::{Namespace, NamespaceUpdater};
use crate::utils::processes::ShellCommand;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Alias {
    #[serde(flatten)]
    name: Identifier,
    desc: String,
    alias: String,
}

impl Alias {
    pub fn new<IntoStr>(name: IntoStr, description: IntoStr, alias: IntoStr) -> Alias
    where
        IntoStr: Into<String>,
    {
        Alias {
            name: Identifier::new(name),
            desc: description.into(),
            alias: alias.into(),
        }
    }
    pub fn namespace(&self) -> Option<&'_ str> {
        self.name.namespace()
    }
    pub fn name(&self) -> &'_ str {
        self.name.name()
    }
    pub fn desc(&self) -> &'_ str {
        self.desc.as_str()
    }
    pub fn alias(&self) -> &'_ str {
        self.alias.as_str()
    }
}

impl NamespaceUpdater for Alias {
    fn update(&mut self, namespace: impl Into<String>) {
        self.name.update(namespace)
    }
}

impl Namespace for &Alias {
    fn namespace(&self) -> Option<&str> {
        self.name.namespace()
    }
}

impl Namespace for Alias {
    fn namespace(&self) -> Option<&str> {
        self.name.namespace()
    }
}

impl Command for &Alias {
    fn command(&self) -> &str {
        self.alias.as_str()
    }
}

impl Command for Alias {
    fn command(&self) -> &str {
        self.alias.as_str()
    }
}

impl Dependencies for &Alias {}
impl Dependencies for Alias {}

impl<'a> Into<String> for &'a Alias {
    fn into(self) -> String {
        format!("{}\t{}", &self.name, &self.desc)
    }
}

impl Into<ShellCommand<String>> for Alias {
    // todo: implement command parsing logic to support pipes and logical symbols etc....
    fn into(self) -> ShellCommand<String> {
        ShellCommand::new(self.alias)
    }
}

impl Display for Alias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "alias {}='{}' # {}", self.name, self.alias, self.desc)
    }
}

#[cfg(test)]
mod tests {
    use super::Alias;
    use crate::core::commands::Command;
    use crate::core::identifiers::Identifier;
    #[test]
    fn test_vars() {
        let alias = Alias::new(
            "test_alias",
            "test_description",
            "some text then {{ var1 }} and so {{var2 }} and after that {{var3}} into {{var_4}}.",
        );
        let expected_vars = vec![
            Identifier::new("{{ var1 }}"),
            Identifier::new("{{var2 }}"),
            Identifier::new("{{var3}}"),
            Identifier::new("{{var_4}}"),
        ];
        let vars: Vec<Identifier> = alias.dependencies();
        assert_eq!(expected_vars, vars);
    }
}
