use crate::config::AppSettings;
use crate::config::ErrorsSettings;
use crate::config_engine::ErrorsConfigEngine;
use crate::environment::ErrorEnvironment;
use cache_engine::ErrorCacheEngine;
use cli::SubCommand;
use sam::core::choices::Choice;
use sam::core::identifiers::Identifier;
use sam_engine::ErrorSamEngine;
use std::collections::HashMap;
use thiserror::Error;

mod cache_engine;
mod cli;
mod config;
mod config_engine;
mod environment;
mod logger;
mod sam_engine;
mod userinterface;
mod vars_cache;

fn main() {
    match run() {
        Ok(i) => std::process::exit(i),
        Err(err) => {
            eprintln!("The application failed to run because {}", err)
        }
    }
}

fn run() -> Result<i32> {
    let cli_request = cli::read_cli_request()?;
    let app_config = AppSettings::load(Some(cli_request.settings))?;
    let environment = environment::from_settings(app_config)?;
    run_command(cli_request.command, environment)
}

fn run_command(sub_command: SubCommand, env: environment::Environment) -> Result<i32> {
    match sub_command {
        SubCommand::SamCommand(s) => Ok(env.sam_engine().run(s)?),
        SubCommand::CacheCommand(s) => Ok(env.cache_engine().run(s)?),
        SubCommand::ConfigCheck(s) => Ok(env.config_engine().run(s)?),
    }
}

type Result<T> = std::result::Result<T, ErrorMain>;

#[derive(Debug, Error)]
pub enum ErrorMain {
    #[error("configuration file contains invalid settings ->\t{0}")]
    Settings(#[from] ErrorsSettings),
    #[error("invalid command line arguments ->\t{0}")]
    Cli(#[from] cli::CLIError),
    #[error("the initialization of the application failed because ->\t{0}")]
    Environment(#[from] ErrorEnvironment),
    #[error("{0}")]
    SamEngine(#[from] ErrorSamEngine),
    #[error("{0}")]
    CacheCommand(#[from] ErrorCacheEngine),
    #[error("{0}")]
    ConfigError(#[from] ErrorsConfigEngine),
}
