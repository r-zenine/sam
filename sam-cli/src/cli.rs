use crate::cache_engine::CacheCommand;
use crate::config_engine::ConfigCommand;
use crate::history_engine::HistoryCommand;
use crate::HashMap;
use clap::{App, Arg, ArgMatches, Values};
use sam_core::engines::SamCommand;
use sam_core::entities::choices::Choice;
use sam_core::entities::identifiers;
use sam_core::entities::identifiers::Identifier;
use std::convert::TryFrom;
use std::env;
use std::ffi::OsString;
use thiserror::Error;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const ABOUT: &str = "sam lets you difine custom aliases and search them using fuzzy search.";
const ABOUT_SUB_RUN: &str = "let's you select and alias then run it";
const ABOUT_SUB_SHOW_HISTORY: &str = "displays the last commands that you ran";
const ABOUT_SUB_RUN_LAST: &str = "runs the last command that was run again. shortcut is `sam %`";
const ABOUT_SUB_SHOW_LAST: &str = "runs the last command that was run again. shortcut is `sam s`";
const ABOUT_SUB_CHECK_CONFIG: &str = "checks your configuration files";
const ABOUT_SUB_CACHE_CLEAR: &str = "clears the cache for vars 'from_command' outputs";
const ABOUT_SUB_CACHE_KEYS: &str = "lists all the cache keys";
const ABOUT_SUB_CACHE_DELETE: &str =
    "explore the content of the command cache in order to delete entries";
const ABOUT_SUB_ALIAS: &str = "run's a provided alias";

#[derive(Clone, Debug, PartialEq)]
pub enum SubCommand {
    SamCommand(SamCommand),
    HistoryCommand(HistoryCommand),
    CacheCommand(CacheCommand),
    ConfigCheck(ConfigCommand),
}
#[derive(Clone, Debug, PartialEq)]
pub struct CLIRequest {
    pub command: SubCommand,
    pub settings: CLISettings,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CLISettings {
    pub dry: bool,
    pub silent: bool,
    pub no_cache: bool,
    pub default_choices: DefaultChoices,
}

impl TryFrom<ArgMatches<'_>> for CLISettings {
    type Error = CLIError;
    fn try_from(matches: ArgMatches) -> Result<Self, Self::Error> {
        let dry = matches.is_present("dry");
        let silent = matches.is_present("silent");
        let no_cache = matches.is_present("no-cache");

        let defaults_extractor = |subcommand: &str| {
            matches
                .subcommand_matches(subcommand)
                .and_then(|e| e.values_of("choices"))
        };

        let defaults_values = matches
            .values_of("choices")
            .or_else(|| defaults_extractor("alias"))
            .or_else(|| defaults_extractor("run"));

        let default_choices = DefaultChoices::try_from(defaults_values)?;

        Ok(CLISettings {
            dry,
            silent,
            no_cache,
            default_choices,
        })
    }
}

fn app_init() -> App<'static, 'static> {
    let arg_choices = Arg::with_name("choices")
        .short("c")
        .long("choices")
        .takes_value(true)
        .multiple(true)
        .help("provide choices for vars. example '-c ns::var=choice'");

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

    let subc_interract_history = App::new("history").about(ABOUT_SUB_SHOW_HISTORY);
    let subc_rerun_last = App::new("run-last").alias("%").about(ABOUT_SUB_RUN_LAST);
    let subc_show_last = App::new("show-last").alias("s").about(ABOUT_SUB_SHOW_LAST);
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
        .arg(arg_choices.clone())
        .subcommand(subc_run)
        .subcommand(subc_alias)
        .subcommand(subc_rerun_last)
        .subcommand(subc_show_last)
        .subcommand(subc_interract_history)
        .subcommand(App::new("check-config").about(ABOUT_SUB_CHECK_CONFIG))
        .subcommand(App::new("cache-clear").about(ABOUT_SUB_CACHE_CLEAR))
        .subcommand(App::new("cache-keys").about(ABOUT_SUB_CACHE_KEYS))
        .subcommand(App::new("cache-keys-delete").about(ABOUT_SUB_CACHE_DELETE))
}

fn make_cli_request<'a, T, I>(app: App<'a, 'a>, args: I) -> Result<CLIRequest, CLIError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = app.get_matches_from(args);

    let settings = CLISettings::try_from(matches.clone())?;

    let command: SubCommand = match matches.subcommand() {
        ("alias", Some(e)) => {
            let alias = parse_alias(e.value_of("alias"))?;
            SubCommand::SamCommand(SamCommand::ExecuteAlias { alias })
        }
        ("run-last", Some(_)) => {
            SubCommand::HistoryCommand(HistoryCommand::ExecuteLastExecutedAlias)
        }
        ("show-last", Some(_)) => {
            SubCommand::HistoryCommand(HistoryCommand::DisplayLastExecutedAlias)
        }
        ("history", Some(_)) => SubCommand::HistoryCommand(HistoryCommand::InterractWithHistory),
        ("check-config", Some(_)) => SubCommand::ConfigCheck(ConfigCommand::All),
        ("cache-clear", Some(_)) => SubCommand::CacheCommand(CacheCommand::Clear),
        ("cache-keys", Some(_)) => SubCommand::CacheCommand(CacheCommand::PrintKeys),
        ("cache-keys-delete", Some(_)) => SubCommand::CacheCommand(CacheCommand::DeleteEntries),

        (&_, _) => SubCommand::SamCommand(SamCommand::ChooseAndExecuteAlias),
    };
    Ok(CLIRequest { command, settings })
}

pub fn read_cli_request() -> Result<CLIRequest, CLIError> {
    let app = app_init();
    make_cli_request(app, &mut env::args_os())
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefaultChoices(pub HashMap<Identifier, Vec<Choice>>);

impl TryFrom<Option<Values<'_>>> for DefaultChoices {
    type Error = CLIError;
    fn try_from(values_o: Option<Values<'_>>) -> Result<Self, Self::Error> {
        let mut default_h = HashMap::default();
        if let Some(values) = values_o {
            for value in values {
                let (id, choice) = parse_choice(value)?;
                default_h.insert(id, vec![choice]);
            }
        }
        Ok(DefaultChoices(default_h))
    }
}

fn parse_alias(alias: Option<&str>) -> Result<Identifier, CLIError> {
    if let Some(a) = alias {
        Ok(identifiers::Identifier::from_str(a))
    } else {
        Err(CLIError::MissingAliasIdentifier)
    }
}

fn parse_choice(default: &str) -> Result<(Identifier, Choice), CLIError> {
    let parts: Vec<&str> = default.split('=').collect();
    if parts.len() == 2 {
        let id = Identifier::from_str(parts[0]);
        if id.namespace.is_none() {
            Err(CLIError::MissingNamespaceForChoice(id, default.to_string()))
        } else {
            let choice = Choice::new(parts[1], None);
            Ok((id, choice))
        }
    } else {
        Err(CLIError::MalformedChoice(default.to_string()))
    }
}

#[derive(Debug, Error)]
pub enum CLIError {
    #[error("the alias identifier that was provided does not exist")]
    MissingAliasIdentifier,
    #[error("The variable name '{0}' does not have a namespace in this section of the command line '{1}'")]
    MissingNamespaceForChoice(Identifier, String),
    #[error("malformed choice {0}, it should be -c namespace::var_name=choice")]
    MalformedChoice(String),
}

#[cfg(test)]
mod tests {

    use crate::cli::DefaultChoices;
    use maplit::hashmap;
    use sam_core::entities::{choices::Choice, identifiers::Identifier};

    use super::{app_init, make_cli_request, CLIRequest, SubCommand};
    use crate::cli::CLISettings;
    use sam_core::engines::SamCommand;

    #[test]
    fn alias_subcommand() {
        let app = app_init();
        let test_string = &[
            "sam",
            "alias",
            "some_namespace::some_alias",
            "-csome_ns::some_choice=value",
            "-csome_ns::some_other_choice=value2",
        ];
        let request = make_cli_request(app, test_string);
        let expected_cli_request = CLIRequest {
            command: SubCommand::SamCommand(SamCommand::ExecuteAlias {
                alias: Identifier::with_namespace("some_alias", Some("some_namespace")),
            }),
            settings: CLISettings {
                dry: false,
                silent: false,
                no_cache: false,
                default_choices: DefaultChoices(hashmap! {
                Identifier::with_namespace("some_choice", Some("some_ns")) => vec![Choice::from_value("value")],
                Identifier::with_namespace("some_other_choice", Some("some_ns")) => vec![Choice::from_value("value2")],
                                }),
            },
        };

        assert_eq!(request.unwrap(), expected_cli_request);
    }

    #[test]
    fn no_subcommand() {
        let app = app_init();
        let test_string = &[
            "sam",
            "-csome_ns::some_choice=value",
            "-csome_ns::some_other_choice=value2",
        ];
        let request = make_cli_request(app, test_string);
        let expected_cli_request = CLIRequest {
            command: SubCommand::SamCommand(SamCommand::ChooseAndExecuteAlias {}),
            settings: CLISettings {
                dry: false,
                silent: false,
                no_cache: false,
                default_choices: DefaultChoices(hashmap! {
                Identifier::with_namespace("some_choice", Some("some_ns")) => vec![Choice::from_value("value")],
                Identifier::with_namespace("some_other_choice", Some("some_ns")) => vec![Choice::from_value("value2")],
                                }),
            },
        };

        assert_eq!(request.unwrap(), expected_cli_request);
    }
    #[test]
    fn run_subcommand() {
        let app = app_init();
        let test_string = &[
            "sam",
            "run",
            "-csome_ns::some_choice=value",
            "-csome_ns::some_other_choice=value2",
        ];
        let request = make_cli_request(app, test_string);
        let expected_cli_request = CLIRequest {
            command: SubCommand::SamCommand(SamCommand::ChooseAndExecuteAlias {}),
            settings: CLISettings {
                dry: false,
                silent: false,
                no_cache: false,
                default_choices: DefaultChoices(hashmap! {
                Identifier::with_namespace("some_choice", Some("some_ns")) => vec![Choice::from_value("value")],
                Identifier::with_namespace("some_other_choice", Some("some_ns")) => vec![Choice::from_value("value2")],
                                }),
            },
        };

        assert_eq!(request.unwrap(), expected_cli_request);
    }
}
