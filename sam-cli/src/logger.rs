use sam_core::entities::aliases::Alias;
use std::{
    cell::RefCell,
    fmt::Display,
    fs::File,
    path::{Path, PathBuf},
};

use std::io::Write;
use thiserror::Error;

use sam_core::engines::SamLogger;

use crate::ErrorMain;

#[derive(Debug)]
pub struct FileLogger {
    file: RefCell<File>,
}

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
    pub fn new(path: &Path) -> Result<Self, ErrorLogger> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
            let file = File::create(path)?;
            let permissions = std::fs::metadata(path)?.permissions();
            if permissions.readonly() {
                return Err(ErrorLogger::FileIsReadOnly(path.to_path_buf()));
            }
            Ok(FileLogger {
                file: RefCell::new(file),
            })
        } else {
            return Err(ErrorLogger::NoLogsUnderRoot(path.to_path_buf()));
        }
    }

    pub fn error(&self, err: ErrorMain) {
        let mut handle = self.file.borrow_mut();
        writeln!(handle, "The application failed to run \n-> {}", err,)
            .expect("Can't write to log file");
    }
}

impl SamLogger for FileLogger {
    fn final_command(&self, alias: &Alias, fc: &dyn Display) {
        let mut handle = self.file.borrow_mut();
        writeln!(
            handle,
            "[SAM][ alias='{}::{}'] Running final command: '{}'",
            alias.namespace().unwrap_or_default(),
            alias.name(),
            fc,
        )
        .expect("Can't write to log file");
    }

    fn command(&self, var: &dyn Display, cmd: &dyn AsRef<str>) {
        let mut handle = self.file.borrow_mut();
        writeln!(
            handle,
            "[SAM][ var = '{}' ] Running: '{}'",
            var,
            cmd.as_ref(),
        )
        .expect("Can't write to log file");
    }

    fn choice(&self, var: &dyn Display, choice: &dyn Display) {
        let mut handle = self.file.borrow_mut();
        writeln!(handle, "[SAM][ var = '{}' ] Choice was: '{}'", var, choice,)
            .expect("Can't write to log file");
    }
    fn alias(&self, alias: &Alias) {
        let mut handle = self.file.borrow_mut();
        writeln!(
            handle,
            "[SAM][ alias = '{}::{}' ]",
            alias.namespace().unwrap_or_default(),
            alias.name(),
        )
        .expect("Can't write to log file");
    }
}

pub struct StdErrLogger;
impl SamLogger for StdErrLogger {
    fn final_command(&self, alias: &Alias, fc: &dyn Display) {
        eprintln!(
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
    fn command(&self, var: &dyn Display, cmd: &dyn AsRef<str>) {
        eprintln!(
            "{}{}[SAM][ var = '{}' ]{} Running: '{}'",
            termion::color::Fg(termion::color::Green),
            termion::style::Bold,
            var,
            termion::style::Reset,
            cmd.as_ref(),
        );
    }
    fn choice(&self, var: &dyn Display, choice: &dyn Display) {
        eprintln!(
            "{}{}[SAM][ var = '{}' ]{} Choice was: '{}'",
            termion::color::Fg(termion::color::Green),
            termion::style::Bold,
            var,
            termion::style::Reset,
            choice,
        );
    }
    fn alias(&self, alias: &Alias) {
        eprintln!(
            "{}{}[SAM][ alias = '{}::{}' ]{}",
            termion::color::Fg(termion::color::Green),
            termion::style::Bold,
            alias.namespace().unwrap_or_default(),
            alias.name(),
            termion::style::Reset,
        );
    }
}

pub struct SilentLogger;
impl SamLogger for SilentLogger {
    fn final_command(&self, _: &Alias, _: &dyn Display) {}
    fn command(&self, _: &dyn Display, _: &dyn AsRef<str>) {}
    fn choice(&self, _: &dyn Display, _: &dyn Display) {}
    fn alias(&self, _: &Alias) {}
}
