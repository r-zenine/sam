use ssam::core::vars::{Choice, Dependencies, ErrorsVarsRepository, VarName};
use ssam::io::readers::{
    read_aliases_from_file, read_scripts, read_vars_repository, ErrorScriptRead, ErrorsAliasRead,
    ErrorsVarRead,
};
use ssam::utils::processes::ShellCommand;
use std::collections::HashMap;
use std::fmt::Display;
use std::process::Command;

mod config;
mod userinterface;

use crate::config::{AppSettings, ErrorsConfig};
use clap::App;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &'static str = "ssa lets you search trough your aliases and one-liner scripts.";
const ABOUT_SUB_RUN: &'static str = "show your aliases and scripts.";
const ABOUT_SUB_BASHRC : &'static str = "output's a collection of aliases definitions into your bashrc. use 'source `ssa bashrc`' in your bashrc file";

const PROMPT: &'_ str = "Choose a script/alias > ";

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
        Err(ErrorsSSAM::UI(userinterface::ErrorsUI::SkimAborted)) => {}
        Err(e) => eprintln!("Could not run the program as expected because {}", e),
        Ok(status) => std::process::exit(status),
    }
}

fn run() -> Result<i32> {
    let cfg = AppSettings::load()?;
    let scripts = read_scripts(cfg.scripts_dir())?;
    let aliases = read_aliases_from_file(cfg.aliases_file())?;
    let vars_repo = read_vars_repository(cfg.vars_file())?;
    let ui_interface = userinterface::UserInterface::default();
    let item = ui_interface.run(PROMPT, aliases, scripts)?;
    match item.kind {
        userinterface::UIItemKind::Script => {
            let script = item.as_script().unwrap().to_owned();
            let mut command: Command = ShellCommand::as_command(script);
            let exit_status = command.status()?;
            return exit_status.code().ok_or(ErrorsSSAM::ExitCode);
        }
        userinterface::UIItemKind::Alias => {
            let alias = item.as_alias().unwrap();
            let exec_seq = vars_repo.execution_sequence(alias)?;
            let choices: HashMap<VarName, Choice> = vars_repo
                .choices(&ui_interface, exec_seq)?
                .into_iter()
                .collect();
            let final_command = alias.substitute_for_choices(&choices).unwrap();
            let mut command: Command = ShellCommand::new(final_command).into();
            let exit_status = command.status()?;
            return exit_status.code().ok_or(ErrorsSSAM::ExitCode);
        }
    }
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
type Result<T> = std::result::Result<T, ErrorsSSAM>;
#[derive(Debug)]
enum ErrorsSSAM {
    ExitCode,
    Config(ErrorsConfig),
    ScriptRead(ErrorScriptRead),
    AliasRead(ErrorsAliasRead),
    VarRead(ErrorsVarRead),
    VarsRepository(ErrorsVarsRepository),
    UI(userinterface::ErrorsUI),
    SubCommand(std::io::Error),
    SubCommandOutput(std::string::FromUtf8Error),
}

impl Display for ErrorsSSAM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "an error occured when ")?;
        match self {
            ErrorsSSAM::Config(e) => writeln!(f, "reading the configuration.\n{}", e),
            ErrorsSSAM::ScriptRead(e) => writeln!(f, "reading the scripts. \n{}", e),
            ErrorsSSAM::AliasRead(e) => writeln!(f, "reading aliases.\n{}", e),
            ErrorsSSAM::VarRead(e) => writeln!(f, "reading vars.\n{}", e),
            ErrorsSSAM::UI(e) => writeln!(f, "launching the terminal user interface\n{:?}", e),
            ErrorsSSAM::SubCommand(e) => writeln!(f, "launching the selected command\n{:?}", e),
            ErrorsSSAM::SubCommandOutput(e) => {
                writeln!(f, "launching the selected command\n{:?}", e)
            }
            ErrorsSSAM::ExitCode => writeln!(f, "trying to return the exit code."),
            ErrorsSSAM::VarsRepository(e) => {
                writeln!(f, "computing figuring out dependencies {:?}.", e)
            }
        }
    }
}

impl From<ErrorsVarsRepository> for ErrorsSSAM {
    fn from(v: ErrorsVarsRepository) -> Self {
        ErrorsSSAM::VarsRepository(v)
    }
}

impl From<ErrorsVarRead> for ErrorsSSAM {
    fn from(v: ErrorsVarRead) -> Self {
        ErrorsSSAM::VarRead(v)
    }
}

impl From<std::string::FromUtf8Error> for ErrorsSSAM {
    fn from(v: std::string::FromUtf8Error) -> Self {
        ErrorsSSAM::SubCommandOutput(v)
    }
}

impl From<std::io::Error> for ErrorsSSAM {
    fn from(v: std::io::Error) -> Self {
        ErrorsSSAM::SubCommand(v)
    }
}

impl From<userinterface::ErrorsUI> for ErrorsSSAM {
    fn from(v: userinterface::ErrorsUI) -> Self {
        ErrorsSSAM::UI(v)
    }
}

impl From<ErrorsAliasRead> for ErrorsSSAM {
    fn from(v: ErrorsAliasRead) -> Self {
        ErrorsSSAM::AliasRead(v)
    }
}
impl From<ErrorsConfig> for ErrorsSSAM {
    fn from(v: ErrorsConfig) -> Self {
        ErrorsSSAM::Config(v)
    }
}

impl From<ErrorScriptRead> for ErrorsSSAM {
    fn from(v: ErrorScriptRead) -> Self {
        ErrorsSSAM::ScriptRead(v)
    }
}
