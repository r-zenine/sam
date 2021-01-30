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
