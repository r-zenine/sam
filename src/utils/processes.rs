use std::ffi::OsStr;
use std::process::Command;

#[derive(Debug)]
pub struct ShellCommand<T> {
    command: T,
}

impl<T> ShellCommand<T> {
    pub fn new(command: T) -> Self {
        Self { command }
    }

    #[allow(dead_code)]
    pub fn as_command<U>(u: U) -> Command
    where
        U: Into<ShellCommand<T>>,
        T: AsRef<OsStr>,
    {
        let sh_cmd: ShellCommand<T> = u.into();
        sh_cmd.into()
    }
}

impl<T> Into<Command> for ShellCommand<T>
where
    T: AsRef<OsStr>,
{
    fn into(self) -> Command {
        let mut command = Command::new("/bin/sh");
        command.arg("-c").arg(self.command);
        let curr_dir = std::env::current_dir();
        if let Ok(dir) = curr_dir {
            command.current_dir(dir);
        }
        command
    }
}
