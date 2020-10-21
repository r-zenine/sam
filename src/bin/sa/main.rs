use ssam::io::readers::{read_aliases_from_file, read_scripts, ErrorAliasRead, ErrorScriptRead};
use std::fmt::Display;
use std::process::Command;

mod config;
mod userinterface;

use crate::config::{AppSettings, ConfigError};

fn main() {
    if let Err(e) = run() {
        eprintln!("Could not run the program as expected because {}", e)
    };
}

fn run() -> Result<()> {
    let cfg = AppSettings::load()?;
    let scripts = read_scripts(cfg.scripts_dir())?;
    let aliases = read_aliases_from_file(cfg.aliases_file())?;
    let ui_interface = userinterface::UserInterface::new()?;
    let mut command: Command = ui_interface.run(aliases, scripts)?.into();
    let output = command.output()?;
    eprint!("{}", String::from_utf8(output.stderr)?);
    print!("{}", String::from_utf8(output.stdout)?);
    Ok(())
}

// Error handling for the sa app.
type Result<T> = std::result::Result<T, SAError>;
#[derive(Debug)]
enum SAError {
    ErrorConfig(ConfigError),
    ErrorScriptRead(ErrorScriptRead),
    ErrorAliasRead(ErrorAliasRead),
    ErrorUI(userinterface::UIError),
    ErrorSubCommand(std::io::Error),
    ErrorSubCommandOutput(std::string::FromUtf8Error),
}

impl From<std::string::FromUtf8Error> for SAError {
    fn from(v: std::string::FromUtf8Error) -> Self {
        SAError::ErrorSubCommandOutput(v)
    }
}

impl From<std::io::Error> for SAError {
    fn from(v: std::io::Error) -> Self {
        SAError::ErrorSubCommand(v)
    }
}

impl From<userinterface::UIError> for SAError {
    fn from(v: userinterface::UIError) -> Self {
        SAError::ErrorUI(v)
    }
}

impl From<ErrorAliasRead> for SAError {
    fn from(v: ErrorAliasRead) -> Self {
        SAError::ErrorAliasRead(v)
    }
}
impl Display for SAError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SAError::ErrorConfig(e) => {
                writeln!(f, "an error occured when reading the configuration.\n{}", e)
            }
            SAError::ErrorScriptRead(e) => {
                writeln!(f, "an error occured when reading the scripts. \n{}", e)
            }
            SAError::ErrorAliasRead(e) => {
                writeln!(f, "an error occured when reading aliases.\n{}", e)
            }
            SAError::ErrorUI(e) => writeln!(
                f,
                "an error occured when launching the terminal user interface\n{:?}",
                e
            ),
            SAError::ErrorSubCommand(e) => writeln!(
                f,
                "an error occured when launching the selected command\n{:?}",
                e
            ),
            SAError::ErrorSubCommandOutput(e) => writeln!(
                f,
                "an error occured when launching the selected command\n{:?}",
                e
            ),
        }
    }
}

impl From<ConfigError> for SAError {
    fn from(v: ConfigError) -> Self {
        SAError::ErrorConfig(v)
    }
}

impl From<ErrorScriptRead> for SAError {
    fn from(v: ErrorScriptRead) -> Self {
        SAError::ErrorScriptRead(v)
    }
}
