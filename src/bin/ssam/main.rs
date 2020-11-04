use ssam::core::aliases::Alias;
use ssam::core::vars::{Choice, Dependencies, ErrorsVarsRepository, VarName, VarsRepository};
use ssam::io::readers::{
    read_aliases_from_file, read_vars_repository, ErrorScriptRead, ErrorsAliasRead, ErrorsVarRead,
};
use ssam::utils::processes::ShellCommand;
use std::collections::HashMap;
use std::fmt::Display;
use std::process::Command;

mod config;
mod userinterface;

use crate::config::{AppSettings, ErrorsConfig};
use clap::{App, Arg};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &'static str =
    "ssam lets you difine custom aliases and search them using fuzzy search.";
const ABOUT_SUB_RUN: &'static str = "show your aliases";
const ABOUT_SUB_ALIAS: &'static str = "run's a provided alias";
const ABOUT_SUB_BASHRC : &'static str = "output's a collection of aliases definitions into your bashrc. use 'source `ssa bashrc`' in your bashrc file";

const PROMPT: &'_ str = "Choose an alias to run > ";

fn main() {
    let matches = App::new("ssam")
        .version(VERSION)
        .author(AUTHORS)
        .about(ABOUT)
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .subcommand(App::new("run").about(ABOUT_SUB_RUN))
        .subcommand(
            App::new("alias")
                .arg(
                    Arg::with_name("alias")
                        .help("the alias to run.")
                        .required(true)
                        .index(1),
                )
                .about(ABOUT_SUB_ALIAS),
        )
        .subcommand(App::new("bashrc").about(ABOUT_SUB_BASHRC))
        .get_matches();
    let result = match matches.subcommand() {
        ("alias", Some(e)) => run_alias(e.value_of("alias").unwrap()),
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
struct AppContext {
    ui_interface: userinterface::UserInterface,
    aliases: Vec<Alias>,
    vars: VarsRepository,
}
impl AppContext {
    fn try_load() -> Result<AppContext> {
        let config = AppSettings::load()?;
        let ui_interface = userinterface::UserInterface::default();
        let aliases = read_aliases_from_file(config.aliases_file())?;
        let vars = read_vars_repository(config.vars_file())?;
        Ok(AppContext {
            ui_interface,
            aliases,
            vars,
        })
    }
}

fn run() -> Result<i32> {
    let ctx = AppContext::try_load()?;
    let item = ctx.ui_interface.select_alias(PROMPT, &ctx.aliases)?;
    let alias = item.alias();
    execute_alias(&ctx, alias)
}

fn run_alias(alias_name: &'_ str) -> Result<i32> {
    let ctx = AppContext::try_load()?;
    let alias = ctx
        .aliases
        .iter()
        .filter(|e| e.name() == alias_name)
        .next()
        .ok_or(ErrorsSSAM::InvalidAliasSelection)?;
    execute_alias(&ctx, alias)
}

fn execute_alias(ctx: &AppContext, alias: &Alias) -> Result<i32> {
    let exec_seq = ctx.vars.execution_sequence(alias)?;
    let choices: HashMap<VarName, Choice> = ctx
        .vars
        .choices(&ctx.ui_interface, exec_seq)?
        .into_iter()
        .collect();
    let final_command = alias.substitute_for_choices(&choices).unwrap();
    let mut command: Command = ShellCommand::new(final_command).into();
    let exit_status = command.status()?;
    exit_status.code().ok_or(ErrorsSSAM::ExitCode)
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
        println!("alias {}='ssam alias {}'", alias.name(), alias.name());
    }
    println!("# STOP SSAM generated aliases:");

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
    InvalidAliasSelection,
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
            ErrorsSSAM::InvalidAliasSelection => {
                writeln!(f, "looking for the requested alias. it was not found.")
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
