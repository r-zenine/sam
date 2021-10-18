use sam_core::aliases::Alias;
use std::fmt::Display;

use crate::sam_engine::SamLogger;

pub struct StdErrLogger;
impl SamLogger for StdErrLogger {
    fn final_command(&self, alias: &Alias, fc: &dyn Display) {
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
