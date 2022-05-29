use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ShellCommand<T: Clone> {
    command: T,
}

fn current_shell_or_sh() -> String {
    env::var("SHELL").unwrap_or_else(|_| String::from("/bin/sh"))
}

impl<T> ShellCommand<T>
where
    T: Clone,
{
    pub fn new(command: T) -> Self {
        Self { command }
    }

    pub fn make_command<U>(u: U) -> Command
    where
        U: Into<ShellCommand<T>>,
        T: AsRef<OsStr>,
    {
        let sh_cmd: ShellCommand<T> = u.into();
        sh_cmd.into()
    }
    pub fn value(&self) -> &T {
        &self.command
    }
}

use lazy_static::lazy_static;
use regex::Regex;

use sam_core::entities::aliases::Alias;

lazy_static! {
    static ref ENVVARRE: Regex = Regex::new(r#"\$\{(?P<var>[a-zA-Z0-9_]+)\}"#).unwrap();
}

impl ShellCommand<String> {
    pub fn replace_env_vars_in_command(
        &self,
        variables: &HashMap<String, String>,
    ) -> std::io::Result<ShellCommand<String>> {
        let replace_pattern = "$$$var".to_string();
        let sanitized = ENVVARRE
            .replace_all(self.command.as_str(), replace_pattern.as_str())
            .to_string();
        let command_escaped = shellwords::escape(&sanitized);
        let s = format!("echo \"{}\"|envsubst", command_escaped);
        let shell_cmd = ShellCommand::<String>::new(s);
        let mut cmd: Command = shell_cmd.into();
        cmd.envs(variables);
        let out = cmd.output()?;
        let new_cmd = String::from_utf8_lossy(out.stdout.as_slice())
            .replace('\n', "")
            .replace('\\', "");
        Ok(ShellCommand::<String>::new(new_cmd))
    }
}

impl From<&'_ str> for ShellCommand<String> {
    fn from(s: &'_ str) -> Self {
        Self::new(s.to_string())
    }
}

impl From<String> for ShellCommand<String> {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<Alias> for ShellCommand<String> {
    fn from(alias: Alias) -> Self {
        ShellCommand::from(alias.alias())
    }
}

impl From<&'_ Alias> for ShellCommand<String> {
    fn from(alias: &Alias) -> Self {
        ShellCommand::from(alias.alias())
    }
}

#[allow(clippy::from_over_into)]
impl<T> Into<Command> for ShellCommand<T>
where
    T: AsRef<OsStr> + Clone,
{
    fn into(self) -> Command {
        let mut command = Command::new(current_shell_or_sh());
        command.arg("-c").arg(self.command);
        command.envs(env::vars());
        let curr_dir = std::env::current_dir();
        if let Ok(dir) = curr_dir {
            command.current_dir(dir);
        }
        command
    }
}

#[cfg(test)]
mod tests {
    use super::ShellCommand;

    #[test]
    fn test_replace_env_vars_in_command() {
        let command = ShellCommand::new(String::from("echo $SOME_VAR"));
        let vars = maplit::hashmap! { String::from("SOME_VAR") => String::from("toto") };
        let output = command
            .replace_env_vars_in_command(&vars)
            .expect("could not replace env vars");
        assert_eq!(output.value(), "echo toto");

        let command = ShellCommand::new(String::from("echo ${SOME_VAR}"));
        let vars = maplit::hashmap! { String::from("SOME_VAR") => String::from("toto") };
        let output = command
            .replace_env_vars_in_command(&vars)
            .expect("could not replace env vars");
        assert_eq!(output.value(), "echo toto");
    }
}
