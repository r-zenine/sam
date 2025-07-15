use std::{fmt::Display, io::BufRead};

use log::debug;
use thiserror::Error;
use tmux_interface::{self, display_message, list_windows, new_window, select_layout, split_window};

pub struct Tmux {
    target_session: String,
}

impl Tmux {
    pub fn current_session_name() -> Result<String, TmuxError> {
            display_message!()
            .print()
            .message("#S")
            .build().into_tmux()
            .output()?
            .stdout()
            .lines()
            .next()
            .transpose()?
            .ok_or_else(TmuxError::NoTmux)
    }

    pub fn with_session(target_session: String) -> Tmux {
        Tmux { target_session }
    }

    pub fn with_current_session() -> Result<Tmux, TmuxError> {
        Self::current_session_name().map(|target_session| Tmux { target_session })
    }

    pub fn list_windows(&self) -> Result<Vec<String>, TmuxError> {
        let output = list_windows!()
            .target_session(&self.target_session)
            .format("#{window_name}")
            .build().into_tmux().output()?.stdout();
        Ok(parsers::parse_list_windows_output(output)?)
    }

    pub fn run_command_in_new_pane(
        &self,
        target_window: &str,
        command: &str,
        directory: &str,
    ) -> Result<bool, TmuxError> {
        if self.list_windows()?.iter().any(|e| *e == target_window) {
            debug!("target window {:?} was found!", target_window);
            let output = split_window!()
                .target_window(target_window)
                .vertical()
                .start_directory(directory)
                .shell_command(command)
                .build()
                .into_tmux().output();
            debug!("executed command {:?} and got output {:?}", command, output);

            Ok(output.map(|out| out.success())?)
        } else {
            debug!(
                "target window {:?} was not found! creating it",
                target_window
            );
            let output = new_window!()
                .window_name(target_window)
                .start_directory(directory)
                .shell_command(command)
                .build()
                .into_tmux().output();

            debug!("executed command {:?} and got output {:?}", command, output);

            Ok(output.map(|out| out.success())?)
        }
    }

    pub fn set_layout(&self, layout: WindowLayout, target_window: &str) -> Result<bool, TmuxError> {
        Ok(select_layout!()
            .target_pane(target_window)
            .layout_name(format!("{}", layout))
            .build().into_tmux()
            .output()
            .map(|out| out.success())?)
    }
}

pub enum WindowLayout {
    Tiled,
    MainVertical,
    MainHorizontal,
    EvenVertical,
    EvenHorizontal,
}

impl Display for WindowLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowLayout::Tiled => f.write_str("tiled"),
            WindowLayout::MainVertical => f.write_str("main-vertical"),
            WindowLayout::MainHorizontal => f.write_str("main-horizontal"),
            WindowLayout::EvenVertical => f.write_str("even-vertical"),
            WindowLayout::EvenHorizontal => f.write_str("even-horizontal"),
        }
    }
}

mod parsers {
    use std::io::BufRead;
    pub fn parse_list_windows_output(stdout: Vec<u8>) -> Result<Vec<String>, std::io::Error> {
        let mut out = Vec::default();
        for line_r in stdout.lines() {
            let line = line_r?;
            out.push(line);
        }
        Ok(out)
    }
}

#[derive(Error, Debug)]
pub enum TmuxError {
    #[error("not running in tmux")]
    NoTmux(),

    #[error("error while interracting with tmux")]
    Tmux(#[from] tmux_interface::Error),

    #[error("error while parsing tmux output {0}")]
    Parsing(#[from] std::io::Error),
}
