use ssam::io::readers::{read_aliases_from_file, read_scripts, ErrorAliasRead, ErrorScriptRead};
use std::fmt::Display;

mod config;

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
    println!("Scripts:");
    for s in scripts {
        println!("{}", s)
    }
    println!("\n\nAliases");
    for a in aliases {
        println!("{}", a)
    }
    Ok(())
}

// Error handling for the sa app.
type Result<T> = std::result::Result<T, SAError>;
#[derive(Debug)]
enum SAError {
    ErrorConfig(ConfigError),
    ErrorScriptRead(ErrorScriptRead),
    ErrorAliasRead(ErrorAliasRead),
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
