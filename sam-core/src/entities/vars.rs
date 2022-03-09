use crate::entities::choices::Choice;
use crate::entities::commands::Command;
use crate::entities::dependencies::Dependencies;
use crate::entities::identifiers::Identifier;
use crate::entities::namespaces::{Namespace, NamespaceUpdater};
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::hash::Hash;

// Var represent a variable with a command that can be used in an crate::core:Alias.
// Var can be static when choices is not empty or dyamic whenthe from_command is not empty
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Var {
    #[serde(flatten)]
    name: Identifier,
    desc: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    from_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    from_input: Option<String>,
}

impl Var {
    /// new creates a new var with a name a description and a static list of choices.
    pub fn new<IntoStr>(name: IntoStr, desc: IntoStr, choices: Vec<Choice>) -> Var
    where
        IntoStr: Into<String>,
    {
        Var {
            name: Identifier::new(name),
            desc: desc.into(),
            choices,
            from_command: None,
            from_input: None,
        }
    }

    /// new creates a new var with a name and a description that will get it's list of choices
    /// from runing the provided command.
    pub fn from_command<IntoStr>(name: IntoStr, desc: IntoStr, from_command: IntoStr) -> Var
    where
        IntoStr: Into<String>,
    {
        Var {
            name: Identifier::new(name),
            desc: desc.into(),
            choices: vec![],
            from_command: Some(from_command.into()),
            from_input: None,
        }
    }

    pub fn from_input<IntoStr>(name: IntoStr, desc: IntoStr, from_input: IntoStr) -> Var
    where
        IntoStr: Into<String>,
    {
        Var {
            name: Identifier::new(name),
            desc: desc.into(),
            choices: vec![],
            from_command: None,
            from_input: Some(from_input.into()),
        }
    }

    pub const fn is_command(&self) -> bool {
        self.from_command.is_some()
    }

    pub const fn is_input(&self) -> bool {
        self.from_input.is_some()
    }

    pub fn name(&self) -> Identifier {
        self.name.clone()
    }

    pub fn choices(&self) -> Vec<Choice> {
        self.choices.clone()
    }

    pub fn prompt(&self) -> Option<&str> {
        self.from_input.as_deref()
    }
}

impl NamespaceUpdater for Var {
    fn update(&mut self, namespace: impl Into<String>) {
        self.name.update(namespace)
    }
}

impl Namespace for Var {
    fn namespace(&self) -> Option<&str> {
        self.name.namespace()
    }
}

impl Namespace for &Var {
    fn namespace(&self) -> Option<&str> {
        self.name.namespace()
    }
}

impl Command for Var {
    fn command(&self) -> &str {
        self.from_command.as_deref().unwrap_or("")
    }
}

impl Command for &Var {
    fn command(&self) -> &str {
        self.from_command.as_deref().unwrap_or("")
    }
}
/// Dependencies returns the dependencies of this variable if it gets it's
/// choices from a command.
///```rust
/// use sam_core::entities::vars::Var;
/// use sam_core::entities::identifiers::Identifier;
/// use sam_core::entities::commands::Command;
/// let example = Var::from_command("name", "description", "ls -l {{ location }} | grep {{pattern}}");
/// assert_eq!(example.dependencies(), vec![Identifier::new("location"), Identifier::new("pattern")]);
///```
impl Dependencies for Var {}
impl Hash for Var {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(&self.name, state)
    }
}

impl PartialEq for Var {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
    }
}

impl Borrow<Identifier> for Var {
    fn borrow(&self) -> &Identifier {
        &self.name
    }
}

impl Eq for Var {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::dependencies::ErrorsResolver;
    use crate::entities::identifiers::fixtures::*;
    use crate::entities::vars::fixtures::*;
    use maplit::hashmap;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    #[test]
    fn test_hashes_are_equal() {
        let mut hasher = DefaultHasher::new();
        let mut hasher_2 = DefaultHasher::new();
        let var_name = VAR_LISTING_NAME.clone();
        let var = VAR_LISTING.clone();
        var_name.hash(&mut hasher);
        var.hash(&mut hasher_2);
        assert_eq!(hasher.finish(), hasher_2.finish());
    }

    #[test]
    fn test_parse_vars() {
        assert_eq!(
            Identifier::parse::<&str>(VAR_LISTING_COMMAND.as_str(), None),
            VAR_LISTING_DEPS.clone(),
        )
    }

    #[test]
    fn test_var_dependencies() {
        assert_eq!(VAR_LISTING.dependencies(), VAR_LISTING_DEPS.clone());
    }

    #[test]
    fn test_substitute_for_choices() {
        // case 1: all is good.
        let choices = hashmap! {
            VAR_DIRECTORY_NAME.clone() => vec![VAR_DIRECTORY_CHOICE_1.clone()],
            VAR_PATTERN_NAME.clone() => vec![VAR_PATTERN_CHOICE_2.clone()],
        };

        let var = VAR_LISTING.clone();
        let r = var.substitute_for_choices(&choices);
        let output = format!(
            "ls -l {} |grep -v {}",
            VAR_DIRECTORY_CHOICE_1.value(),
            VAR_PATTERN_CHOICE_2.value()
        );
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), vec![output]);
        // case 2: we are missing a var choice.
        let missing_choices = hashmap! {
            VAR_PATTERN_NAME.clone() => vec![VAR_PATTERN_CHOICE_2.clone()],
        };
        let r2 = var.substitute_for_choices(&missing_choices);
        assert!(r2.is_err());
        match r2.unwrap_err() {
            ErrorsResolver::NoChoiceWasAvailable(name) => {
                assert_eq!(name, VAR_DIRECTORY_NAME.clone())
            }
            _ => unreachable!(),
        }
    }
}

pub mod fixtures {
    use super::*;
    use crate::entities::identifiers::fixtures::*;
    use lazy_static::lazy_static;
    lazy_static! {
        pub static ref VAR_USE_LISTING_COMMAND: String =
            String::from("cat {{listing}} |grep -v {{ns::pattern}}");
        pub static ref VAR_USE_LISTING_DESC: String = String::from(
            "output element in {{listing}} and discards everything that matches {{pattern}}",
        );
        pub static ref VAR_USE_LISTING_CHOICES: Vec<Choice> = vec![];
        pub static ref VAR_USE_LISTING_DEPS: Vec<Identifier> =
            vec![Identifier::new("listing"), Identifier::new("pattern")];
        pub static ref VAR_USE_LISTING: Var = Var {
            name: VAR_USE_LISTING_NAME.clone(),
            from_command: Some(VAR_USE_LISTING_COMMAND.clone()),
            desc: VAR_USE_LISTING_DESC.clone(),
            choices: VAR_USE_LISTING_CHOICES.clone(),
            from_input: None,
        };
        pub static ref VAR_LISTING_COMMAND: String =
            String::from("ls -l {{directory}} |grep -v {{ ns::pattern }}");
        pub static ref VAR_LISTING_DESC: String = String::from(
            "list element in {{directory}} and discards everything that matches {{pattern}}",
        );
        pub static ref VAR_LISTING_CHOICES: Vec<Choice> = vec![];
        pub static ref VAR_LISTING_DEPS: Vec<Identifier> = vec![
            Identifier::new("directory"),
            Identifier::with_namespace("pattern", Some("ns")),
        ];
        pub static ref VAR_LISTING: Var = Var {
            name: VAR_LISTING_NAME.clone(),
            from_command: Some(VAR_LISTING_COMMAND.clone()),
            desc: VAR_LISTING_DESC.clone(),
            choices: VAR_LISTING_CHOICES.clone(),
            from_input: None,
        };
        pub static ref VAR_DIRECTORY_DESC: String =
            String::from("A list of safe directory paths where to perform commands.");
        pub static ref VAR_DIRECTORY_CHOICE_1: Choice =
            Choice::new("/var/log", Some("logs directory"));
        pub static ref VAR_DIRECTORY_CHOICE_2: Choice =
            Choice::new("/home", Some("users directory"));
        pub static ref VAR_DIRECTORY_CHOICES: Vec<Choice> = vec![
            VAR_DIRECTORY_CHOICE_1.clone(),
            VAR_DIRECTORY_CHOICE_2.clone(),
        ];
        pub static ref VAR_DIRECTORY: Var = Var {
            name: VAR_DIRECTORY_NAME.clone(),
            from_command: None,
            desc: VAR_DIRECTORY_DESC.clone(),
            choices: VAR_DIRECTORY_CHOICES.clone(),
            from_input: None,
        };
        pub static ref VAR_PATTERN_DESC: String = String::from("A black list of patterns");
        pub static ref VAR_PATTERN_CHOICE_1: Choice =
            Choice::new("service", Some("service pattern"));
        pub static ref VAR_PATTERN_CHOICE_2: Choice =
            Choice::new("ryad", Some("users ryad pattern"));
        pub static ref VAR_PATTERN_CHOICES: Vec<Choice> =
            vec![VAR_PATTERN_CHOICE_1.clone(), VAR_PATTERN_CHOICE_2.clone()];
        pub static ref VAR_PATTERN: Var = Var {
            name: VAR_PATTERN_NAME.clone(),
            from_command: None,
            desc: VAR_PATTERN_DESC.clone(),
            choices: VAR_PATTERN_CHOICES.clone(),
            from_input: None,
        };
        pub static ref VAR_MISSING_COMMAND: String =
            String::from("ls -l {{directory}} |grep -v {{pattern2}}");
        pub static ref VAR_MISSING_DESC: String = String::from(
            "list element in {{directory}} and discards everything that matches {{pattern}}",
        );
        pub static ref VAR_MISSING_CHOICES: Vec<Choice> = vec![];
        pub static ref VAR_MISSING_DEPS: Vec<Identifier> =
            vec![Identifier::new("directory"), Identifier::new("pattern2")];
        pub static ref VAR_MISSING: Var = Var {
            name: VAR_MISSING_NAME.clone(),
            from_command: Some(VAR_MISSING_COMMAND.clone()),
            desc: VAR_MISSING_DESC.clone(),
            choices: VAR_MISSING_CHOICES.clone(),
            from_input: None,
        };
    }
}
