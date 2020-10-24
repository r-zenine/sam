use ssam::io::readers::{read_aliases_from_file, read_scripts, ErrorAliasRead, ErrorScriptRead};
use std::fmt::Display;
use std::process::Command;

mod config;
mod userinterface;

use crate::config::{AppSettings, ConfigError};
use clap::App;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &'static str = "ssa lets you search trough your aliases and one-liner scripts.";
const ABOUT_SUB_RUN: &'static str = "show your aliases and scripts.";
const ABOUT_SUB_BASHRC : &'static str = "output's a collection of aliases definitions into your bashrc. use 'source `ssa bashrc`' in your bashrc file";

fn main() {
    let matches = App::new("ssam")
        .version(VERSION)
        .author(AUTHORS)
        .about(ABOUT)
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .subcommand(App::new("run").about(ABOUT_SUB_RUN))
        .subcommand(App::new("bashrc").about(ABOUT_SUB_BASHRC))
        .get_matches();
    let result = match matches.subcommand() {
        ("run", Some(_)) => run(),
        ("bashrc", Some(_)) => bashrc(),
        (&_, _) => {
            println!("{}", matches.usage());
            Ok(0)
        }
    };
    match result {
        Err(e) => eprintln!("Could not run the program as expected because {}", e),
        Ok(status) => std::process::exit(status),
    }
}

fn run() -> Result<i32> {
    let cfg = AppSettings::load()?;
    let scripts = read_scripts(cfg.scripts_dir())?;
    let aliases = read_aliases_from_file(cfg.aliases_file())?;
    let ui_interface = userinterface::UserInterface::new()?;
    let mut command: Command = ui_interface.run(aliases, scripts)?.into();
    let exit_status = command.status()?;
    exit_status.code().ok_or(SAError::ErrorExitCode)
}

fn bashrc() -> Result<i32> {
    let cfg = AppSettings::load()?;
    let aliases = read_aliases_from_file(cfg.aliases_file())?;
    println!("# *************** IMPORTANT *******************");
    println!("#                                             *");
    println!("# Put the following line in your (bash/zsh)rc *");
    println!("#                                             *");
    println!("# eval \"$(ssam bashrc)\"                       *");
    println!("#                                             *");
    println!("# *********************************************");
    println!("# START SSAM generated aliases:");
    println!("alias am='ssam run'");
    for alias in aliases {
        println!("{}", alias);
    }
    println!("# STOP SSAM generated aliases:");

    println!("export PATH=$PATH:{}", cfg.scripts_dir().display());
    Ok(0)
}

// Error handling for the sa app.
type Result<T> = std::result::Result<T, SAError>;
#[derive(Debug)]
enum SAError {
    ErrorExitCode,
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
            SAError::ErrorExitCode => {
                writeln!(f, "an error occured when trying to return the exit code.")
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
