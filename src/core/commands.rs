use std::collections::HashSet;

use crate::core::identifiers::Identifier;
use crate::core::namespaces::Namespace;
use comma::Command as CmdParser;
use lazy_static::lazy_static;
use regex::Regex;
use std::str::FromStr;

lazy_static! {
    // matches the following patters :
    // - $ENV_VAR39
    // - $(ENV_VAR39)
    // - ${ENV_VAR39}
    static ref ENVVARRE: Regex = Regex::new("\\$[\\{\\(]?(?P<env_var>[a-zA-Z0-9_]+)[\\}\\)]?").unwrap();
    static ref SUBCMD_RE: Regex = Regex::new("`+(?P<sub_cmd>[a-zA-Z0-9_]+)`+").unwrap();
    static ref SUBCMD_NESTED_RE: Regex = Regex::new("[\"']+(?P<sub_nest>[^'\"]+)[\"']+").unwrap();
}

pub trait Command: Namespace {
    // Returns a string representation of a command
    fn command(&self) -> &str;
    // Returns the dependencies of an command.
    fn dependencies(&self) -> Vec<Identifier> {
        Identifier::parse(self.command(), self.namespace())
    }
    fn env_vars(&self) -> Vec<&str> {
        extract_env_vars(self.command())
    }
}

fn extract_env_vars(input: &str) -> Vec<&str> {
    ENVVARRE
        .captures_iter(input)
        .flat_map(|e| e.name("env_var"))
        .map(|e| e.as_str())
        .collect()
}

pub fn unset_env_vars<'a, T>(commands: impl Iterator<Item = &'a T>) -> HashSet<String>
where
    T: Command + 'a,
{
    let env_vars: HashSet<String> = std::env::vars().map(|(key, _)| key).collect();
    let set: HashSet<String> = commands
        .flat_map(|e| e.env_vars())
        .map(|e| e.to_string())
        .collect();

    set.difference(&env_vars).map(|e| e.to_string()).collect()
}

pub fn programs_used<'a, T>(commands: impl Iterator<Item = &'a T>) -> HashSet<String>
where
    T: Command + 'a,
{
    commands
        .flat_map(|e| extract_programs_from_command(e.command()))
        .collect()
}

fn extract_programs_from_command(cmd: &str) -> Vec<String> {
    let cmd = SUBCMD_NESTED_RE.replace_all(cmd, "").to_string();

    cmd.split("&&")
        .flat_map(|s| s.split("||"))
        .flat_map(|s| s.split("|"))
        .chain(
            SUBCMD_RE
                .captures_iter(cmd.as_str())
                .flat_map(|c| c.name("sub_cmd"))
                .map(|c| c.as_str()),
        )
        .flat_map(|s| {
            if let Ok(parsed_cmd) = CmdParser::from_str(s) {
                Some(parsed_cmd.name)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {

    use crate::core::commands::Command;
    use crate::core::commands::{extract_env_vars, unset_env_vars};
    use crate::core::namespaces::Namespace;
    #[test]
    fn test_extract_env_vars() {
        let result: Vec<&'static str> = vec!["Some_VAR"];

        let example = "echo $Some_VAR";
        assert_eq!(extract_env_vars(example), result);

        let example = "echo $(Some_VAR)";
        assert_eq!(extract_env_vars(example), result);

        let example = "echo ${Some_VAR}";
        assert_eq!(extract_env_vars(example), result);
    }

    #[test]
    fn test_unset_env_vars() {
        let commands = vec![StringCommand::from_str("$SOME_CRAZY_ENV_VAR")];
        let unsets = unset_env_vars(commands.iter());
        assert_eq!(unsets.len(), 1);
        assert!(unsets.contains("SOME_CRAZY_ENV_VAR"));
    }

    #[test]
    fn extract_programs_from_command() {
        let rslt = super::extract_programs_from_command(
            "some_program arg1 ar2|grep toto `sub_cmd` |yq \"toto|tata\" 'titi ouou' || some_text and && grep -l ",
        );
        assert_eq!(
            vec! {"some_program", "grep", "yq", "some_text", "grep", "sub_cmd",},
            rslt
        )
    }

    struct StringCommand {
        _command: String,
    }

    impl StringCommand {
        fn from_str(cmd: &str) -> Self {
            StringCommand {
                _command: cmd.to_string(),
            }
        }
    }

    impl Namespace for StringCommand {
        fn namespace(&self) -> Option<&str> {
            Some(self._command.as_str())
        }
    }
    impl Command for StringCommand {
        fn command(&self) -> &str {
            self._command.as_str()
        }
    }
}
