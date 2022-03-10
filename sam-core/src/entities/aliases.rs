use crate::entities::choices::Choice;
use crate::entities::commands::Command;
use crate::entities::dependencies::Dependencies;
use crate::entities::dependencies::ErrorsResolver;
use crate::entities::identifiers::Identifier;
use crate::entities::namespaces::Namespace;
use crate::entities::namespaces::NamespaceUpdater;
use crate::entities::processes::ShellCommand;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;

lazy_static! {
    // matches the following patters :
    // - {{ some_name_1 }}
    // - {{some_name_1 }}
    // - {{ some_name_1}}
    pub static ref VARS_NO_NS_RE: Regex = Regex::new("\\{\\{ ?(?P<vars>[a-zA-Z0-9_]+) ?\\}\\}").unwrap();
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

    pub fn with_choices(
        &self,
        choices: &HashMap<Identifier, Vec<Choice>>,
    ) -> Result<ResolvedAlias, ErrorsResolver> {
        let res = self.substitute_for_choices(choices)?;
        Ok(ResolvedAlias {
            name: self.name.clone(),
            desc: self.desc.clone(),
            original_alias: self.alias.clone(),
            resolved_aliases: res,
            choices: choices.clone(),
        })
    }

    pub fn with_partial_choices(&self, choices: &HashMap<Identifier, Choice>) -> Alias {
        let res = self.substitute_for_choices_partial(choices);

        Alias {
            name: self.name.clone(),
            desc: self.desc.clone(),
            alias: res,
        }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedAlias {
    name: Identifier,
    desc: String,
    original_alias: String,
    resolved_aliases: Vec<String>,
    choices: HashMap<Identifier, Vec<Choice>>,
}

impl ResolvedAlias {
    pub const fn new(
        name: Identifier,
        desc: String,
        original_alias: String,
        resolved_aliases: Vec<String>,
        choices: HashMap<Identifier, Vec<Choice>>,
    ) -> Self {
        ResolvedAlias {
            name,
            desc,
            original_alias,
            resolved_aliases,
            choices,
        }
    }

    pub fn commands(&self) -> &[String] {
        self.resolved_aliases.as_slice()
    }

    pub fn choice(&self, identifier: &Identifier) -> Option<Vec<Choice>> {
        self.choices.get(identifier).map(Clone::clone)
    }

    pub const fn name(&self) -> &Identifier {
        &self.name
    }

    pub fn desc(&self) -> &str {
        &self.desc
    }

    pub const fn choices(&self) -> &HashMap<Identifier, Vec<Choice>> {
        &self.choices
    }
    pub fn original_alias(&self) -> &str {
        &self.original_alias
    }
    pub fn resolved_alias(&self) -> &[String] {
        &self.resolved_aliases
    }
}

impl From<ResolvedAlias> for Alias {
    fn from(r_alias: ResolvedAlias) -> Self {
        Alias {
            name: r_alias.name,
            desc: r_alias.desc,
            alias: r_alias.original_alias,
        }
    }
}

impl Namespace for &ResolvedAlias {
    fn namespace(&self) -> Option<&str> {
        self.name.namespace()
    }
}

impl Namespace for ResolvedAlias {
    fn namespace(&self) -> Option<&str> {
        self.name.namespace()
    }
}

impl Display for ResolvedAlias {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        writeln!(
            f,
            "{}{}Alias:{} {}",
            termion::color::Fg(termion::color::LightCyan),
            termion::style::Bold,
            termion::style::Reset,
            self.name,
        )?;
        writeln!(
            f,
            "{}{}Choices:{}\n",
            termion::color::Fg(termion::color::LightCyan),
            termion::style::Bold,
            termion::style::Reset,
        )?;

        for (choice, values) in &self.choices {
            write!(
                f,
                "\t{}{}{} =\t",
                termion::style::Bold,
                choice,
                termion::style::Reset
            )?;
            for val in values {
                write!(f, "{} ", val)?;
            }
            writeln!(f)?;
        }
        writeln!(
            f,
            "\n{}{}{}Executed commands:{}",
            termion::color::Fg(termion::color::LightCyan),
            termion::style::Bold,
            termion::style::Italic,
            termion::style::Reset,
        )?;
        for cmd in &self.resolved_aliases {
            writeln!(f, "\t- {}", cmd)?;
        }
        Ok(())
    }
}

#[allow(clippy::from_over_into)]
impl<'a> Into<String> for &'a Alias {
    fn into(self) -> String {
        format!("{}\t{}", &self.name, &self.desc)
    }
}

#[allow(clippy::from_over_into)]
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
    use crate::entities::aliases::Alias;
    use crate::entities::identifiers::fixtures::*;
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
    use crate::entities::commands::Command;
    use crate::entities::identifiers::Identifier;
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
