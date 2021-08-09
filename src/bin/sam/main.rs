use crate::vars_cache::{NoopVarsCache, RocksDBVarsCache, VarsCache};
use clap::Values;
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
use clap::{App, Arg, ArgMatches};

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
    let command_line_args = app_init().get_matches();
    let app_ctx = AppContext::from_command_line_args(&command_line_args)
        .expect("Can't initialize the application");

    let result = match command_line_args.subcommand() {
        ("alias", Some(e)) => run_alias(app_ctx, e.value_of("alias").unwrap()),
        ("bashrc", Some(_)) => bashrc(),
        ("check-config", Some(_)) => check_config(app_ctx),
        ("clear-cache", Some(_)) => clear_cache(),
        (&_, _) => run(app_ctx),
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
    // TODO Todo remove user interface from the context
    ui_interface: userinterface::UserInterface,
    aliases: AliasesRepository,
    vars: VarsRepository,
    silent: bool, // TODO provide a custom logger implementation
    dry: bool,    // TODO provide a custom executor
    env_variables: HashMap<String, String>,
}

impl AppContext {
    fn from_command_line_args(matches: &ArgMatches) -> Result<AppContext> {
        let dry = matches.is_present("dry");
        let silent = matches.is_present("silent");
        let no_cache = matches.is_present("no-cache");
        let choices = matches.values_of("choices").or(matches
            .subcommand_matches("alias")
            .and_then(|e| e.values_of("choices")));
        let defaults = Self::parse_defaults(choices);
        let config = AppSettings::load()?;
        let cache: Box<dyn VarsCache> = if !no_cache {
            Box::new(RocksDBVarsCache::new(config.cache_dir(), &config.ttl()))
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
        println!("{:?}", &defaults);
        vars.set_defaults(&defaults)?;
        let aliases = AliasesRepository::new(aliases_vec.into_iter())?;
        vars.ensure_no_missing_dependency()?;
        Ok(AppContext {
            ui_interface,
            aliases,
            vars,
            dry,
            silent,
            env_variables: config.variables(),
        })
    }

    fn parse_defaults(defaults: Option<Values>) -> HashMap<Identifier, Choice> {
        let mut default_h = HashMap::default();
        println!("{:?}", defaults);
        if let Some(values) = defaults {
            for value in values {
                println!("{:?}", value);
                if let Some((id, choice)) = Self::parse_default(value) {
                    default_h.insert(id, choice);
                }
            }
        }
        default_h
    }

    fn parse_default(default: &str) -> Option<(Identifier, Choice)> {
        let parts: Vec<&str> = default.split("=").collect();
        if parts.len() == 2 {
            let id = Identifier::from_str(parts[0]);
            let choice = Choice::new(parts[1], None);
            Some((id, choice))
        } else {
            None
        }
    }
}

fn run(mut ctx: AppContext) -> Result<i32> {
    let item = ctx
        .ui_interface
        .select_alias(PROMPT, &ctx.aliases.aliases())?;
    let alias = item.alias();
    execute_alias(&ctx, alias)
}

fn run_alias(ctx: AppContext, input: &'_ str) -> Result<i32> {
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
        command.envs(&ctx.env_variables);
        let exit_status = command.status()?;
        exit_status.code().ok_or(Errorssam::ExitCode)
    } else {
        Ok(0)
    }
}

fn clear_cache() -> Result<i32> {
    let config = AppSettings::load()?;
    let cache = RocksDBVarsCache::new(config.cache_dir(), &config.ttl());
    cache.clear_cache().map(|_| 0).map_err(Errorssam::VarsCache)
}

fn check_config(ctx: AppContext) -> Result<i32> {
    let missing_envvars_in_aliases = unset_env_vars(ctx.aliases.aliases().iter());
    let missing_envvars_in_vars = unset_env_vars(ctx.vars.vars_iter());
    let envvars_in_config: HashSet<&String> = ctx.env_variables.keys().collect();
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
        .arg(
            Arg::with_name("choices")
                .short("c")
                .long("choices")
                .takes_value(true)
                .multiple(true)
                .help("provide choices for vars"),
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
                .arg(
                    Arg::with_name("choices")
                        .short("c")
                        .long("choices")
                        .takes_value(true)
                        .multiple(true)
                        .help("provide choices for vars"),
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
