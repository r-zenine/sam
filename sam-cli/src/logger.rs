use log::info;
use sam_core::entities::aliases::Alias;
use std::{fmt::Display, path::PathBuf};

use thiserror::Error;

use sam_core::engines::SamLogger;

#[derive(Debug)]
pub struct FileLogger {}

#[derive(Debug, Error)]
pub enum ErrorLogger {
    #[error("Logging directly under / is not supported")]
    NoLogsUnderRoot(PathBuf),
    #[error("provided filepath `{0}` is read only")]
    FileIsReadOnly(PathBuf),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
impl FileLogger {
    pub fn new() -> Self {
        FileLogger {}
    }
}

impl SamLogger for FileLogger {
    fn final_command(&self, alias: &Alias, fc: &dyn Display) {
        info!(
            "[SAM][ alias='{}::{}'] Running final command: '{}'",
            alias.namespace().unwrap_or_default(),
            alias.name(),
            fc,
        )
    }

    fn command(&self, var: &dyn Display, cmd: &dyn AsRef<str>) {
        info!("[SAM][ var = '{}' ] Running: '{}'", var, cmd.as_ref(),)
    }

    fn choice(&self, var: &dyn Display, choice: &dyn Display) {
        info!("[SAM][ var = '{}' ] Choice was: '{}'", var, choice)
    }
    fn alias(&self, alias: &Alias) {
        info!(
            "[SAM][ alias = '{}::{}' ]",
            alias.namespace().unwrap_or_default(),
            alias.name(),
        )
    }
}

pub struct SilentLogger;
impl SamLogger for SilentLogger {
    fn final_command(&self, _: &Alias, _: &dyn Display) {}
    fn command(&self, _: &dyn Display, _: &dyn AsRef<str>) {}
    fn choice(&self, _: &dyn Display, _: &dyn Display) {}
    fn alias(&self, _: &Alias) {}
}
