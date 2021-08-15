use crate::cache_engine::CacheCommand;
use crate::config_engine::ConfigCommand;
use crate::sam_engine::SamCommand;
use crate::Choice;
use crate::HashMap;
use crate::Identifier;
use clap::{App, Arg, ArgMatches, Values};
use sam::core::identifiers;
use std::env;
use std::ffi::OsString;
use thiserror::Error;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const ABOUT: &str = "sam lets you difine custom aliases and search them using fuzzy search.";
const ABOUT_SUB_RUN: &str = "show your aliases";
const ABOUT_SUB_CHECK_CONFIG: &str = "checks your configuration files";
const ABOUT_SUB_CACHE_CLEAR: &str = "clears the cache for vars 'from_command' outputs";
const ABOUT_SUB_CACHE_KEYS: &str = "lists all the cache keys";
const ABOUT_SUB_ALIAS: &str = "run's a provided alias";

#[derive(Clone, Debug)]
pub struct DefaultChoices(pub HashMap<Identifier, Choice>);

#[derive(Clone, Debug)]
pub enum SubCommand {
    SamCommand(SamCommand),
    CacheCommand(CacheCommand),
    ConfigCheck(ConfigCommand),
}
#[derive(Clone, Debug)]
pub struct CLIRequest {
    pub command: SubCommand,
    pub settings: CLISettings,
}

#[derive(Clone, Debug)]
pub struct CLISettings {
    pub dry: bool,
    pub silent: bool,
    pub no_cache: bool,
    pub default_choices: DefaultChoices,
}

impl From<ArgMatches<'_>> for CLISettings {
    fn from(matches: ArgMatches) -> Self {
        let dry = matches.is_present("dry");
        let silent = matches.is_present("silent");
        let no_cache = matches.is_present("no-cache");
        let defaults: DefaultChoices = matches
            .values_of("choices")
            .or_else(|| {
                matches
                    .subcommand_matches("alias")
                    .and_then(|e| e.values_of("choice"))
            })
            .into();
        CLISettings {
            dry,
            silent,
            no_cache,
            default_choices: defaults,
        }
    }
}

fn app_init() -> App<'static, 'static> {
    let arg_choices = Arg::with_name("choices")
        .short("c")
        .long("choices")
        .takes_value(true)
        .multiple(true)
        .help("provide choices for vars");

    let arg_dry = Arg::with_name("dry")
        .long("dry")
        .short("d")
        .help("dry run, don't execute the final command.");

    let arg_silent = Arg::with_name("silent")
        .long("silent")
        .short("s")
        .help("don't cache the output of `from_command` vars.");

    let arg_no_cache = Arg::with_name("no-cache")
        .long("no-cache")
        .short("-n")
        .help("avoid relying of the vars cache.");

    let subc_run = App::new("run")
        .arg(arg_choices.clone())
        .about(ABOUT_SUB_RUN);

    let subc_alias = App::new("alias")
        .arg(
            Arg::with_name("alias")
                .help("the alias to run.")
                .required(true)
                .index(1),
        )
        .arg(arg_choices.clone())
        .about(ABOUT_SUB_ALIAS);

    App::new("sam")
        .version(VERSION)
        .author(AUTHORS)
        .about(ABOUT)
        .arg(arg_dry)
        .arg(arg_silent)
        .arg(arg_no_cache)
        .subcommand(subc_run)
        .subcommand(subc_alias)
        .subcommand(App::new("check-config").about(ABOUT_SUB_CHECK_CONFIG))
        .subcommand(App::new("cache-clear").about(ABOUT_SUB_CACHE_CLEAR))
        .subcommand(App::new("cache-keys").about(ABOUT_SUB_CACHE_KEYS))
}

fn make_cli_request<'a, T, I>(app: App<'a, 'a>, args: I) -> Result<CLIRequest, CLIError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = app.get_matches_from(args);
    let settings: CLISettings = matches.clone().into();

    let command: SubCommand = match matches.subcommand() {
        ("alias", Some(e)) => {
            let alias = parse_alias(e.value_of("alias"))?;
            SubCommand::SamCommand(SamCommand::ExecuteAlias { alias })
        }
        ("check-config", Some(_)) => SubCommand::ConfigCheck(ConfigCommand::CheckUnsetEnvVars),
        ("cache-clear", Some(_)) => SubCommand::CacheCommand(CacheCommand::PrintKeys),
        ("cache-keys", Some(_)) => SubCommand::CacheCommand(CacheCommand::Clear),
        (&_, _) => SubCommand::SamCommand(SamCommand::ChooseAndExecuteAlias),
    };
    Ok(CLIRequest { command, settings })
}

pub fn read_cli_request() -> Result<CLIRequest, CLIError> {
    let app = app_init();
    make_cli_request(app, &mut env::args_os())
}

impl From<Option<Values<'_>>> for DefaultChoices {
    fn from(values_o: Option<Values<'_>>) -> Self {
        let mut default_h = HashMap::default();
        if let Some(values) = values_o {
            for value in values {
                if let Some((id, choice)) = parse_choice(value) {
                    default_h.insert(id, choice);
                }
            }
        }
        DefaultChoices(default_h)
    }
}

fn parse_alias(alias: Option<&str>) -> Result<Identifier, CLIError> {
    if let Some(a) = alias {
        Ok(identifiers::Identifier::from_str(a))
    } else {
        Err(CLIError::MissingAliasIdentifier)
    }
}

fn parse_choice(default: &str) -> Option<(Identifier, Choice)> {
    let parts: Vec<&str> = default.split('=').collect();
    if parts.len() == 2 {
        let id = Identifier::from_str(parts[0]);
        let choice = Choice::new(parts[1], None);
        Some((id, choice))
    } else {
        None
    }
}

#[derive(Debug, Error)]
pub enum CLIError {
    #[error("the alias identifier that was provided does not exist")]
    MissingAliasIdentifier,
}
