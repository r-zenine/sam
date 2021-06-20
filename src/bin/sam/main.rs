use crate::vars_cache::{NoopVarsCache, RocksDBVarsCache, VarsCache};
use sam::core::aliases::Alias;
use sam::core::aliases_repository::{AliasesRepository, ErrorsAliasesRepository};
use sam::core::choices::Choice;
use sam::core::commands::unset_env_vars;
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
use std::collections::HashSet;
use std::process::Command;
use thiserror::Error;

mod config;
mod userinterface;
mod vars_cache;

use crate::config::{AppSettings, ErrorsConfig};
use clap::{App, Arg};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const ABOUT: &str = "sam lets you difine custom aliases and search them using fuzzy search.";
const ABOUT_SUB_RUN: &str = "show your aliases";
const ABOUT_SUB_CHECK_CONFIG: &str = "checks your configuration files";
const ABOUT_SUB_CLEAR_CACHE: &str = "clears the cache for vars 'from_command' outputs";
const ABOUT_SUB_ALIAS: &str = "run's a provided alias";
const ABOUT_SUB_BASHRC : &str = "output's a collection of aliases definitions into your bashrc. use 'source `ssa bashrc`' in your bashrc file";

const PROMPT: &str = "Choose an alias to run > ";

fn main() {
    let matches = app_init().get_matches();
    let dry = matches.is_present("dry");
    let silent = matches.is_present("silent");
    let no_cache = matches.is_present("no-cache");
    let result = match matches.subcommand() {
        ("alias", Some(e)) => run_alias(e.value_of("alias").unwrap(), dry, silent, no_cache),
        ("bashrc", Some(_)) => bashrc(),
        ("check-config", Some(_)) => check_config(),
        ("clear-cache", Some(_)) => clear_cache(),
        (&_, _) => run(dry, silent, no_cache),
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
    aliases: AliasesRepository,
    vars: VarsRepository,
    silent: bool,
    dry: bool,
    variables: HashMap<String, String>,
}

impl AppContext {
    fn try_load(dry: bool, silent: bool, no_cache: bool) -> Result<AppContext> {
        let config = AppSettings::load()?;
        let cache: Box<dyn VarsCache> = if !no_cache {
            Box::new(RocksDBVarsCache::new(config.cache_dir(), &config.ttl())?)
        } else {
            Box::new(NoopVarsCache {})
        };
        let ui_interface = userinterface::UserInterface::new(silent, config.variables(), cache)?;
        let files = walk_dir(config.root_dir())?;
        let mut aliases_vec = vec![];
        let mut vars = VarsRepository::default();
        for f in files {
            if let Some(file_name) = f.file_name() {
                if file_name == "aliases.yaml" || file_name == "aliases.yml" {
                    aliases_vec.extend(read_aliases_from_path(f.as_path())?);
                } else if file_name == "vars.yaml" || file_name == "vars.yml" {
                    vars.merge(read_vars_repository(f.as_path())?);
                }
            }
        }
        let aliases = AliasesRepository::new(aliases_vec.into_iter())?;
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

fn run(dry: bool, silent: bool, no_cache: bool) -> Result<i32> {
    let mut ctx = AppContext::try_load(dry, silent, no_cache)?;
    let item = ctx
        .ui_interface
        .select_alias(PROMPT, &ctx.aliases.aliases())?;
    let alias = item.alias();
    execute_alias(&ctx, alias)
}

fn run_alias(input: &'_ str, dry: bool, silent: bool, no_cache: bool) -> Result<i32> {
    let ctx = AppContext::try_load(dry, silent, no_cache)?;
    let mut elems: Vec<&str> = input.split("::").collect();
    let name = elems.pop().unwrap_or_default();
    let namespace = elems.pop();
    let alias_ls = ctx.aliases.aliases();
    let alias = alias_ls
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

fn clear_cache() -> Result<i32> {
    let config = AppSettings::load()?;
    let cache = RocksDBVarsCache::new(config.cache_dir(), &config.ttl())?;
    cache.clear_cache().map(|_| 0).map_err(Errorssam::VarsCache)
}

fn check_config() -> Result<i32> {
    let ctx = AppContext::try_load(false, true, false)?;
    let missing_envvars_in_aliases = unset_env_vars(ctx.aliases.aliases().iter());
    let missing_envvars_in_vars = unset_env_vars(ctx.vars.vars_iter());
    let envvars_in_config: HashSet<&String> = ctx.variables.keys().collect();
    let all_envvars: HashSet<&String> = missing_envvars_in_vars
        .union(&missing_envvars_in_aliases)
        .collect();

    let missing_envvars: Vec<_> = all_envvars.difference(&envvars_in_config).collect();
    if missing_envvars.len() == 0 {
        return Ok(0);
    }
    println!("Undifined environement variables:");
    for var in &missing_envvars {
        println!(
            "- {}{}{}{}",
            termion::style::Bold,
            termion::color::Fg(termion::color::Red),
            var,
            termion::style::Reset,
        );
    }
    Ok(1)
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

fn app_init() -> App<'static, 'static> {
    App::new("sam")
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
                .help("don't cache the output of `from_command` vars."),
        )
        .arg(
            Arg::with_name("no-cache")
                .long("no-cache")
                .short("-n")
                .help("avoid relying of the vars cache."),
        )
        .subcommand(App::new("run").about(ABOUT_SUB_RUN))
        .subcommand(App::new("check-config").about(ABOUT_SUB_CHECK_CONFIG))
        .subcommand(App::new("clear-cache").about(ABOUT_SUB_CLEAR_CACHE))
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
    #[error("could not figure out alias substitution\n-> {0}")]
    AliasRepository(#[from] ErrorsAliasesRepository),
    #[error("could not initialize the cache\n-> {0}")]
    VarsCache(#[from] vars_cache::CacheError),
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
