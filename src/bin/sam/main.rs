use sam::core::aliases::Alias;
use sam::core::choices::Choice;
use sam::core::dependencies::Dependencies;
use sam::core::identifiers::Identifier;
use sam::core::vars_repository::{ErrorsVarsRepository, VarsRepository};
use sam::io::readers::{
    read_aliases_from_path, read_vars_repository, ErrorsAliasRead, ErrorsVarRead,
};
use sam::utils::fsutils;
use sam::utils::fsutils::walk_dir;
use sam::utils::processes::ShellCommand;
use std::collections::HashMap;
use std::process::Command;
use thiserror::Error;

mod config;
mod userinterface;

use crate::config::{AppSettings, ErrorsConfig};
use clap::{App, Arg};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &str = "sam lets you difine custom aliases and search them using fuzzy search.";
const ABOUT_SUB_RUN: &str = "show your aliases";
const ABOUT_SUB_ALIAS: &str = "run's a provided alias";
const ABOUT_SUB_BASHRC : &str = "output's a collection of aliases definitions into your bashrc. use 'source `ssa bashrc`' in your bashrc file";

const PROMPT: &str = "Choose an alias to run > ";

fn main() {
    let matches = App::new("sam")
        .version(VERSION)
        .author(AUTHORS)
        .about(ABOUT)
        .arg(
            Arg::with_name("dry")
                .long("dry")
                .short("d")
                .help("dry run, don't execute the final command."),
        )
        .arg(
            Arg::with_name("silent")
                .long("silent")
                .short("s")
                .help("avoid outputing logs to the standard output."),
        )
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
    let dry = matches.is_present("dry");
    let silent = matches.is_present("silent");
    let result = match matches.subcommand() {
        ("alias", Some(e)) => run_alias(e.value_of("alias").unwrap(), dry, silent),
        ("bashrc", Some(_)) => bashrc(),
        (&_, _) => run(dry, silent),
    };
    match result {
        Err(Errorssam::UI(userinterface::ErrorsUI::SkimAborted)) => {}
        Err(e) => eprintln!(
            "{}{}Could not run the program as expected because:{}\n-> {}",
            termion::style::Bold,
            termion::color::Fg(termion::color::Red),
            termion::style::Reset,
            e
        ),
        Ok(status) => std::process::exit(status),
    }
}
struct AppContext {
    ui_interface: userinterface::UserInterface,
    aliases: Vec<Alias>,
    vars: VarsRepository,
    silent: bool,
    dry: bool,
    variables: HashMap<String, String>,
}

impl AppContext {
    fn try_load(dry: bool, silent: bool) -> Result<AppContext> {
        let config = AppSettings::load()?;
        let ui_interface = userinterface::UserInterface::new(silent, config.variables())?;
        let files = walk_dir(config.root_dir())?;
        let mut aliases = vec![];
        let mut vars = VarsRepository::default();
        for f in files {
            if let Some(file_name) = f.file_name() {
                if file_name == "aliases.yaml" || file_name == "aliases.yml" {
                    aliases.extend(read_aliases_from_path(f.as_path())?);
                } else if file_name == "vars.yaml" || file_name == "vars.yml" {
                    vars.merge(read_vars_repository(f.as_path())?);
                }
            }
        }
        vars.ensure_no_missing_dependency()?;
        Ok(AppContext {
            ui_interface,
            aliases,
            vars,
            dry,
            silent,
            variables: config.variables(),
        })
    }
}

fn run(dry: bool, silent: bool) -> Result<i32> {
    let mut ctx = AppContext::try_load(dry, silent)?;
    let item = ctx.ui_interface.select_alias(PROMPT, &ctx.aliases)?;
    let alias = item.alias();
    execute_alias(&ctx, alias)
}

fn run_alias(input: &'_ str, dry: bool, silent: bool) -> Result<i32> {
    let ctx = AppContext::try_load(dry, silent)?;
    let mut elems: Vec<&str> = input.split("::").collect();
    let name = elems.pop().unwrap_or_default();
    let namespace = elems.pop();
    let alias = ctx
        .aliases
        .iter()
        .find(|e| e.name() == name && e.namespace() == namespace)
        .ok_or(Errorssam::InvalidAliasSelection)?;
    execute_alias(&ctx, alias)
}

fn execute_alias(ctx: &AppContext, alias: &Alias) -> Result<i32> {
    let exec_seq = ctx.vars.execution_sequence(alias)?;
    let choices: HashMap<Identifier, Choice> = ctx
        .vars
        .choices(&ctx.ui_interface, exec_seq)?
        .into_iter()
        .collect();
    let final_command = alias.substitute_for_choices(&choices).unwrap();
    if !ctx.silent {
        logs::final_command(alias, &final_command);
    }
    if !ctx.dry {
        let mut command: Command = ShellCommand::new(final_command).into();
        command.envs(&ctx.variables);
        let exit_status = command.status()?;
        exit_status.code().ok_or(Errorssam::ExitCode)
    } else {
        Ok(0)
    }
}

fn bashrc() -> Result<i32> {
    let cfg = AppSettings::load()?;
    let files = walk_dir(cfg.root_dir())?;
    let mut aliases = vec![];
    for f in files {
        if let Some(file_name) = f.file_name() {
            if file_name == "aliases.yaml" {
                aliases.extend(read_aliases_from_path(f.as_path())?);
            }
        }
    }
    println!("# *************** IMPORTANT *******************");
    println!("#                                             *");
    println!("# Put the following line in your (bash/zsh)rc *");
    println!("#                                             *");
    println!("# eval \"$(sam bashrc)\"                       *");
    println!("#                                             *");
    println!("# *********************************************");
    println!("# START sam generated aliases:");
    println!("alias am='sam run'");
    for alias in aliases {
        println!(
            "alias {}_{}='sam alias {}::{}'",
            alias.namespace().unwrap_or_default(),
            alias.name(),
            alias.namespace().unwrap_or_default(),
            alias.name()
        );
    }
    println!("# STOP sam generated aliases:");

    Ok(0)
}

// Error handling for the sa app.
type Result<T> = std::result::Result<T, Errorssam>;
#[derive(Debug, Error)]
enum Errorssam {
    #[error("could not return an exit code.")]
    ExitCode,
    #[error("could not read the configuration file\n-> {0}")]
    Config(#[from] ErrorsConfig),
    #[error("could not read aliases\n-> {0}")]
    AliasRead(#[from] ErrorsAliasRead),
    #[error("could not read vars\n-> {0}")]
    VarRead(#[from] ErrorsVarRead),
    #[error("could not figure out dependencies\n-> {0}")]
    VarsRepository(#[from] ErrorsVarsRepository),
    #[error("could not run the terminal user interface\n-> {0}")]
    UI(#[from] userinterface::ErrorsUI),
    #[error("could not run a command\n-> {0}")]
    SubCommand(#[from] std::io::Error),
    #[error("could not read a command output\n-> {0}")]
    SubCommandOutput(#[from] std::string::FromUtf8Error),
    #[error("the requested alias was not found")]
    InvalidAliasSelection,
    #[error("filesystem related error\n-> {0}")]
    FilesLookup(#[from] fsutils::ErrorsFS),
}

mod logs {
    use sam::core::aliases::Alias;
    use std::fmt::Display;
    pub fn final_command(alias: &Alias, fc: impl Display) {
        println!(
            "{}{}[SAM][ alias='{}::{}']{} Running final command: {}{}'{}'{}",
            termion::color::Fg(termion::color::Green),
            termion::style::Bold,
            alias.namespace().unwrap_or_default(),
            alias.name(),
            termion::style::Reset,
            termion::color::Fg(termion::color::Green),
            termion::style::Bold,
            fc,
            termion::style::Reset,
        );
    }
}
