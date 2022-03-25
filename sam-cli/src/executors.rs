use std::collections::HashMap;

use sam_core::engines::{ErrorSamEngine, SamExecutor};
use sam_core::entities::{aliases::ResolvedAlias, processes::ShellCommand};

pub struct ShellExecutor {}

impl SamExecutor for ShellExecutor {
    fn execute_resolved_alias(
        &self,
        alias: &ResolvedAlias,
        env_variables: &HashMap<String, String>,
    ) -> Result<i32, ErrorSamEngine> {
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
