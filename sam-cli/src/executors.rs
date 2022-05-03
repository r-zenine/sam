use std::collections::HashMap;
use std::env;
use std::rc::Rc;

use log::debug;
use sam_core::engines::{ErrorSamEngine, SamExecutor};
use sam_core::entities::{aliases::ResolvedAlias, processes::ShellCommand};
use sam_terminals::tmux::{Tmux, TmuxError};

pub fn make_executor(dry: bool) -> Result<Rc<dyn SamExecutor>, Box<dyn std::error::Error>> {
    if dry {
        Ok(Rc::new(DryExecutor {}))
    } else if env::var("TMUX").is_ok() {
        debug!("running inside tmux, using TmuxExecutor");
        let executor = TmuxExecutor::with_current_session()?;
        Ok(Rc::new(executor))
    } else {
        debug!("no tmux detected, using ShellExecutor");
        Ok(Rc::new(ShellExecutor {}))
    }
}

pub struct TmuxExecutor {
    current_session: String,
    windows: Vec<String>,
}

impl TmuxExecutor {
    fn with_current_session() -> Result<Self, TmuxError> {
        let current_session = Tmux::current_session_name()?;
        let windows = Tmux::with_session(current_session.clone()).list_windows()?;
        Ok(TmuxExecutor {
            current_session,
            windows,
        })
    }

    fn window_name_for_alias(&self, alias: &ResolvedAlias) -> String {
        let mut idx = 0;
        let alias_name = format!("{}", alias.name()).replace(':', "_");
        for name in &self.windows {
            if name.starts_with(&alias_name) {
                idx += 1
            }
        }
        format!("{}-{}", alias_name, idx + 1)
    }
}

impl SamExecutor for TmuxExecutor {
    fn execute_resolved_alias(
        &self,
        alias: &ResolvedAlias,
        env_variables: &HashMap<String, String>,
    ) -> Result<i32, ErrorSamEngine> {
        println!();
        eprintln!();
        let window_name = self.window_name_for_alias(alias);
        let directory = env::current_dir()?;
        let t = Tmux::with_session(self.current_session.clone());
        let commands = alias.commands();
        if commands.len() == 1 {
            ShellExecutor {}.execute_resolved_alias(alias, env_variables)
        } else {
            for cmd in alias.commands() {
                let shcmd =
                    ShellCommand::new(cmd.clone()).replace_env_vars_in_command(env_variables)?;
                let command = shcmd.value();
                debug!("execute_resolved_alias: running command {:?}", cmd);
                t.run_command_in_new_pane(&window_name, command, directory.to_str().unwrap_or("."))
                    .map_err(|err| ErrorSamEngine::ExecutorFailure(Box::new(err)))?;
                t.set_layout(sam_terminals::tmux::WindowLayout::Tiled, &window_name)
                    .map_err(|err| ErrorSamEngine::ExecutorFailure(Box::new(err)))?;
            }
            t.set_layout(sam_terminals::tmux::WindowLayout::Tiled, &window_name)
                .map_err(|err| ErrorSamEngine::ExecutorFailure(Box::new(err)))?;
            Ok(0)
        }
    }
}

pub struct ShellExecutor {}

impl SamExecutor for ShellExecutor {
    fn execute_resolved_alias(
        &self,
        alias: &ResolvedAlias,
        env_variables: &HashMap<String, String>,
    ) -> Result<i32, ErrorSamEngine> {
        println!();
        eprintln!();
        for cmd in alias.commands() {
            let mut command: std::process::Command = ShellCommand::new(cmd).into();
            command.envs(env_variables);
            let exit_status = command.status()?;
            exit_status.code().ok_or(ErrorSamEngine::ExitCode)?;
        }
        Ok(0)
    }
}

pub struct DryExecutor {}
impl SamExecutor for DryExecutor {
    fn execute_resolved_alias(
        &self,
        _alias: &ResolvedAlias,
        _env_variables: &HashMap<String, String>,
    ) -> Result<i32, ErrorSamEngine> {
        Ok(0)
    }
}
