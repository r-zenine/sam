use std::collections::HashMap;

use crate::sam_engine::{self, ErrorSamEngine, SamExecutor};
use sam::{
    core::{aliases::ResolvedAlias, commands::Command},
    utils::processes::ShellCommand,
};

pub struct ShellExecutor {}

impl SamExecutor for ShellExecutor {
    fn execute_resolved_alias(
        &self,
        alias: &ResolvedAlias,
        env_variables: &HashMap<String, String>,
    ) -> sam_engine::Result<i32> {
        let mut command: std::process::Command = ShellCommand::new(alias.command()).into();
        command.envs(env_variables);
        let exit_status = command.status()?;
        exit_status.code().ok_or(ErrorSamEngine::ExitCode)
    }
}

pub struct DryExecutor {}
impl SamExecutor for DryExecutor {
    fn execute_resolved_alias(
        &self,
        _alias: &ResolvedAlias,
        _env_variables: &HashMap<String, String>,
    ) -> sam_engine::Result<i32> {
        Ok(0)
    }
}

#[cfg(test)]
pub mod mocks {
    use std::{cell::RefCell, collections::HashMap};

    use sam::core::aliases::ResolvedAlias;

    use crate::sam_engine::{self, SamExecutor};

    #[derive(Default)]
    pub struct LogExecutor {
        pub commands: RefCell<Vec<(ResolvedAlias, HashMap<String, String>)>>,
    }

    impl SamExecutor for LogExecutor {
        fn execute_resolved_alias(
            &self,
            alias: &ResolvedAlias,
            env_variables: &HashMap<String, String>,
        ) -> sam_engine::Result<i32> {
            let mut cmd_mut = self.commands.borrow_mut();
            cmd_mut.push((alias.clone(), env_variables.to_owned()));
            Ok(0)
        }
    }
}
