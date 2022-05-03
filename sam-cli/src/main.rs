use crate::config::{AppSettings, ErrorsSettings};
use crate::config_engine::ErrorsConfigEngine;
use crate::environment::ErrorEnvironment;
use cache_engine::ErrorCacheEngine;
use cli::SubCommand;
use flexi_logger::{FileSpec, Logger, LoggerHandle, WriteMode};
use history_engine::ErrorHistoryEngine;
use log::error;
use sam_core::engines::ErrorSamEngine;
use std::collections::HashMap;
use thiserror::Error;

mod cache_engine;
mod cli;
mod config;
mod config_engine;
mod environment;
mod executors;
mod history_engine;
mod logger;

fn main() {
    let _logger = init_logger().expect("can't initialize logs");
    match run() {
        Ok(i) => {
            std::process::exit(i);
        }
        Err(err) => {
            eprintln!("An error happened while running the program {}", err);
            std::process::exit(1);
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
        SubCommand::HistoryCommand(s) => Ok(env.history_engine().run(s)?),
    }
}

fn init_logger() -> Result<LoggerHandle> {
    Ok(Logger::try_with_env()?
        .log_to_file(
            FileSpec::default()
        )
        .write_mode(WriteMode::BufferAndFlush)
        .use_utc()
        .start()?)
}

type Result<T> = std::result::Result<T, ErrorMain>;

#[derive(Debug, Error)]
pub enum ErrorMain {
    #[error("Configuration file contains invalid settings \n-> {0}")]
    Settings(#[from] ErrorsSettings),
    #[error("Invalid command line arguments\n->  {0}")]
    Cli(#[from] cli::CLIError),
    #[error("the initialization of the application failed because \n-> {0}")]
    Environment(#[from] ErrorEnvironment),
    #[error("{0}")]
    SamEngine(#[from] ErrorSamEngine),
    #[error("{0}")]
    CacheCommand(#[from] ErrorCacheEngine),
    #[error("{0}")]
    ConfigError(#[from] ErrorsConfigEngine),
    #[error("{0}")]
    HistoryError(#[from] ErrorHistoryEngine),
    #[error("Can't initialise logging because {0}")]
    LoggingError(#[from] flexi_logger::FlexiLoggerError),
}
