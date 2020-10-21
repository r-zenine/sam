use crate::utils::processes::ShellCommand;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    name: String,
    description: Option<String>,
    path: PathBuf,
}

impl Script {
    #[allow(dead_code)]
    pub fn new<S, SS, P>(name: S, description: Option<SS>, path: P) -> Self
    where
        S: Into<String>,
        SS: Into<String>,
        P: Into<PathBuf>,
    {
        Self {
            name: name.into(),
            description: description.map(|e| e.into()),
            path: path.into(),
        }
    }

    pub fn name(&self) -> &'_ str {
        self.name.as_ref()
    }
    pub fn description(&self) -> Option<&'_ str> {
        self.description.as_ref().map(|s| s.as_str())
    }
    pub fn path(&self) -> &'_ Path {
        self.path.as_ref()
    }
}

impl<'a> Into<String> for &'a Script {
    fn into(self) -> String {
        format!(
            "{}\t{}",
            &self.name,
            &self.description.as_deref().unwrap_or("")
        )
    }
}

impl Display for Script {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Script: name={} \t path={} \t description={}",
            self.name(),
            self.path().display(),
            self.description().unwrap_or("not provided")
        )
    }
}

impl Into<ShellCommand<PathBuf>> for Script {
    fn into(self) -> ShellCommand<PathBuf> {
        ShellCommand::new(self.path)
    }
}
