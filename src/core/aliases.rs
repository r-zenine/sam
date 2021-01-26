use crate::core::commands::Command;
use crate::core::dependencies::Dependencies;
use crate::core::identifiers::Identifier;
use crate::core::namespaces::{Namespace, NamespaceUpdater};
use crate::utils::processes::ShellCommand;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::Display;

lazy_static! {
    // matches the following patters :
    // - {{ some_name_1 }}
    // - {{some_name_1 }}
    // - {{ some_name_1}}
    pub static ref VARS_NO_NS_RE: Regex = Regex::new("\\{\\{ ?(?P<vars>[a-zA-Z0-9_]+) ?\\}\\}").unwrap();
}

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

    pub fn update(&mut self, alias: String) {
        self.alias = alias;
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
    pub fn sanitized_alias(&self) -> String { 
        Self::sanitize(self.alias(), self.namespace().unwrap_or(""))
    }
    pub fn identifier(&self) -> Identifier {
        self.name.clone()
    }
    pub fn full_name(&self) -> Cow<'_, str> {
        let n = self.name();
        if let Some(ns) = self.namespace() {
            let full_name = format!("{}::{}", ns, n);
            Cow::Owned(full_name)
        } else {
            Cow::Borrowed(n)
        }
    }

    fn sanitize(alias_def: &str, namespace: &str) -> String {
        let replace_pattern = format!("{{{{ {}::$vars }}}}", namespace);
        VARS_NO_NS_RE
            .replace_all(alias_def, replace_pattern.as_str())
            .to_string()
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

pub mod fixtures {
    use crate::core::aliases::Alias;
    use crate::core::identifiers::fixtures::*;
    use lazy_static::lazy_static;

    lazy_static! {
        pub static ref ALIAS_LS_DIR: Alias = Alias {
            name: ALIAS_LS_DIR_NAME.clone(),
            desc: String::from("some desc"),
            alias: String::from("ls {{ directory }}"),
        };
        pub static ref ALIAS_GREP_DIR: Alias = Alias {
            name: ALIAS_GREP_DIR_NAME.clone(),
            desc: String::from("some desc"),
            alias: String::from("[[ dirs::list ]]|grep {{ pattern }}"),
        };
        pub static ref ALIAS_GREP_DIR_NO_NS: Alias = Alias {
            name: ALIAS_GREP_DIR_NAME.clone(),
            desc: String::from("some desc"),
            alias: String::from("[[ list ]]| grep {{ pattern }}"),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::Alias;
    use crate::core::commands::Command;
    use crate::core::identifiers::Identifier;
    #[test]
    fn vars() {
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

    #[test]
    fn sanitize() {
        let output = Alias::sanitize("{{ super }} no {{ ns::toto }}", "sup");
        assert_eq!("{{ sup::super }} no {{ ns::toto }}", output.as_str());
    }
}
