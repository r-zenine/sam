use crate::entities::namespaces::{Namespace, NamespaceUpdater};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;

lazy_static! {
    // matches the following patters :
    // - {{ some_name_1 }}
    // - {{some_name_1 }}
    // - {{ some_name_1}}
    static ref VARSRE: Regex = Regex::new("(?P<vars>\\{\\{ ?[a-zA-Z0-9_:]+ ?\\}\\})").unwrap();
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Identifier {
    #[serde(rename(serialize = "name", deserialize = "name"))]
    pub inner: String,
    pub namespace: Option<String>,
}

impl Identifier {
    /// new creates an new Identifier object and it will sanitize the input.
    ///```rust
    /// use sam_core::entities::identifiers::Identifier;
    /// use sam_core::entities::namespaces::Namespace;
    /// let var = Identifier::new("{{ pattern }}");
    /// assert_eq!(var.name(), "pattern");
    /// let var = Identifier::new("{{ pattern}}");
    /// assert_eq!(var.name(), "pattern");
    /// let var = Identifier::new("{{pattern }}");
    /// assert_eq!(var.name(), "pattern");
    ///```
    pub fn new<IntoStr>(name: IntoStr) -> Identifier
    where
        IntoStr: Into<String>,
    {
        Identifier {
            inner: name
                .into()
                .replace("{{ ", "{{")
                .replace(" }}", "}}")
                .replace(" ", "")
                .replace("{{", "")
                .replace("}}", ""),
            namespace: None,
        }
    }
    /// new creates an new Identifier object and it will sanitize the input.
    ///```rust
    /// use sam_core::entities::identifiers::Identifier;
    /// use sam_core::entities::namespaces::Namespace;
    /// let var = Identifier::with_namespace("{{ pattern }}", Some("ns"));
    /// assert_eq!(var.name(), "pattern");
    /// assert_eq!(var.namespace(), Some("ns"));
    /// let var = Identifier::with_namespace("{{ pattern}}", Some("ns"));
    /// assert_eq!(var.name(), "pattern");
    /// assert_eq!(var.namespace(), Some("ns"));
    /// let var = Identifier::with_namespace("{{pattern }}", Some("ns"));
    /// assert_eq!(var.name(), "pattern");
    /// assert_eq!(var.namespace(), Some("ns"));
    ///```
    pub fn with_namespace(
        name: impl Into<String>,
        namespace: Option<impl Into<String>>,
    ) -> Identifier {
        Identifier {
            inner: Identifier::sanitize_identifier(name.into()),
            namespace: namespace.map(Into::into),
        }
    }
    /// Dependencies returns the dependencies of this variable if it gets it's
    /// choices from a command.
    ///```rust
    /// use sam_core::entities::identifiers::Identifier;
    /// use sam_core::entities::commands::Command;
    /// let example = Identifier::parse::<&str>("ls -l {{ location }} | grep {{pattern}}", None);
    /// assert_eq!(example, vec![Identifier::new("location"), Identifier::new("pattern")]);
    ///```
    pub fn parse<IntoStr>(s: &str, namespace: Option<IntoStr>) -> Vec<Identifier>
    where
        IntoStr: Into<String> + Clone,
    {
        let default_namespace = namespace.map(Into::<String>::into);
        VARSRE
            .captures_iter(s)
            .map(|e| Identifier::maybe_namespace(e["vars"].to_owned()))
            .map(|(name, ns)| {
                Identifier::with_namespace(name.as_str(), ns.or_else(|| default_namespace.clone()))
            })
            .collect()
    }

    pub fn name(&self) -> &str {
        self.inner.as_str()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(id: &str) -> Identifier {
        let (name, namespace) = Self::maybe_namespace(id);
        Self::with_namespace(name, namespace)
    }

    pub fn maybe_namespace<IntoStr>(str: IntoStr) -> (String, Option<String>)
    where
        IntoStr: Into<String>,
    {
        let s = str.into();
        if s.contains("::") {
            let parts: Vec<&str> = s.split("::").take(2).collect();
            let name = Identifier::sanitize_identifier(parts[1].to_string());
            let namespace = Identifier::sanitize_identifier(parts[0].to_string());
            if !namespace.is_empty() {
                return (name, Some(namespace));
            } else {
                return (name, None);
            }
        }
        (s, None)
    }
    fn sanitize_identifier(s: String) -> String {
        s.replace("{ ", "{")
            .replace(" }", "}")
            .replace(" ", "")
            .replace("{{", "")
            .replace("}}", "")
            .replace("]]", "")
            .replace("[[", "")
    }
}

impl PartialEq<&Identifier> for Identifier {
    fn eq(&self, other: &&Identifier) -> bool {
        other.inner == self.inner
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.namespace().is_some() && self.namespace().unwrap() != "" {
            write!(
                f,
                "{}::{}",
                self.namespace.as_deref().unwrap_or(""),
                self.inner
            )
        } else {
            write!(f, "{}", self.inner)
        }
    }
}

impl NamespaceUpdater for Identifier {
    fn update(&mut self, namespace: impl Into<String>) {
        self.namespace = Some(Into::into(namespace));
    }
}

impl Namespace for Identifier {
    fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }
}

#[derive(Debug, PartialEq)]
pub struct Identifiers(pub Vec<Identifier>);
impl Display for Identifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for id in &self.0 {
            writeln!(f, "- {}", id)?;
        }
        Ok(())
    }
}

pub mod fixtures {
    use super::*;
    use lazy_static::lazy_static;
    lazy_static! {
        pub static ref VAR_USE_LISTING_NAME: Identifier = Identifier::new("use_listing");
        pub static ref VAR_LISTING_NAME: Identifier = Identifier::new("listing");
        pub static ref VAR_DIRECTORY_NAME: Identifier = Identifier::new("directory");
        pub static ref VAR_PATTERN_NAME: Identifier =
            Identifier::with_namespace("pattern", Some("ns"));
        pub static ref VAR_PATTERN_2_NAME: Identifier = Identifier::new("pattern2");
        pub static ref VAR_MISSING_NAME: Identifier = Identifier::new("missing");
        pub static ref ALIAS_LS_DIR_NAME: Identifier =
            Identifier::with_namespace("list", Some("dirs"));
        pub static ref ALIAS_GREP_DIR_NAME: Identifier =
            Identifier::with_namespace("grep", Some("dirs"));
    }
}

#[cfg(test)]
mod tests {
    use super::Identifier;
    #[test]
    fn test_identifier_new() {
        let cases: Vec<(Identifier, &'static str)> = vec![
            (Identifier::new("{{ toto}}"), "toto"),
            (Identifier::new("{{toto }}"), "toto"),
            (Identifier::new("{{toto}}"), "toto"),
        ];
        for (case, result) in cases {
            assert_eq!(&case.inner, result);
        }
    }

    #[test]
    fn test_identifier_from_str() {
        assert_eq!(
            Identifier::from_str("aws::ec2 instance_ip"),
            Identifier::with_namespace("ec2 instance_ip", Some("aws"))
        );
        assert_eq!(
            Identifier::from_str("::ec2_instance_ip"),
            Identifier::new("ec2_instance_ip")
        )
    }
}
