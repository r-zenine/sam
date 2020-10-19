use crate::utils::processes::ShellCommand;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Alias {
    name: String,
    description: String,
    alias: String,
}

impl Alias {
    #[allow(dead_code)]
    pub fn new<IntoStr>(name: IntoStr, description: IntoStr, alias: IntoStr) -> Alias
    where
        IntoStr: Into<String>,
    {
        Alias {
            name: name.into(),
            description: description.into(),
            alias: alias.into(),
        }
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
        writeln!(
            f,
            "alias {}='{}' # {}",
            self.name, self.alias, self.description
        )
    }
}
