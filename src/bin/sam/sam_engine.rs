use sam::core::aliases::Alias;
use sam::core::aliases_repository::{AliasesRepository, ErrorsAliasesRepository};
use sam::core::choices::Choice;
use sam::core::commands::Command;
use sam::core::dependencies::{ErrorsResolver, Resolver};
use sam::core::identifiers::Identifier;
use sam::core::vars_repository::{ErrorsVarsRepository, VarsRepository};
use sam::utils::processes::ShellCommand;
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;
use thiserror::Error;

const PROMPT: &str = "Choose an alias to run > ";

#[derive(Clone, Debug, PartialEq)]
pub enum SamCommand {
    ChooseAndExecuteAlias,
    ExecuteAlias { alias: Identifier },
}

pub struct SamEngine<R: Resolver> {
    pub resolver: R,
    pub aliases: AliasesRepository,
    pub vars: VarsRepository,
    pub logger: Rc<dyn SamLogger>,
    pub env_variables: HashMap<String, String>,
    pub dry: bool,
}

impl<R: Resolver> SamEngine<R> {
    pub fn run(&self, command: SamCommand) -> Result<i32> {
        match command {
            SamCommand::ChooseAndExecuteAlias => self.choose_and_execute_alias(),
            SamCommand::ExecuteAlias { alias } => self.execute_alias(&alias),
        }
    }

    fn choose_and_execute_alias(&self) -> Result<i32> {
        let id = self.aliases.select_alias(&self.resolver, PROMPT)?;
        self.run_alias(id)
    }

    fn execute_alias(&self, alias_id: &Identifier) -> Result<i32> {
        let alias = self.aliases.get(alias_id)?;
        self.run_alias(alias)
    }

    fn run_alias(&self, alias: &Alias) -> Result<i32> {
        let exec_seq = self.vars.execution_sequence(alias)?;
        let choices: HashMap<Identifier, Choice> = self
            .vars
            .choices(&self.resolver, exec_seq)?
            .into_iter()
            .collect();

        let final_alias = alias.with_choices(&choices).unwrap();
        self.logger.final_command(alias, &final_alias.command());
        if !self.dry {
            let mut command: std::process::Command =
                ShellCommand::new(final_alias.command()).into();
            command.envs(&self.env_variables);
            let exit_status = command.status()?;
            exit_status.code().ok_or(ErrorSamEngine::ExitCode)
        } else {
            Ok(0)
        }
    }
}

pub trait SamLogger {
    fn final_command(&self, alias: &Alias, fc: &dyn Display);
    fn command(&self, var: &dyn Display, cmd: &dyn AsRef<str>);
    fn choice(&self, var: &dyn Display, choice: &dyn Display);
    fn alias(&self, alias: &Alias);
}

type Result<T> = std::result::Result<T, ErrorSamEngine>;

#[derive(Debug, Error)]
pub enum ErrorSamEngine {
    #[error("could not return an exit code.")]
    ExitCode,
    #[error("the requested alias was not found")]
    InvalidAliasSelection,
    #[error("could not resolve the dependency because\n-> {0}")]
    Resolver(#[from] ErrorsResolver),
    #[error("could not figure out dependencies\n-> {0}")]
    VarsRepository(#[from] ErrorsVarsRepository),
    #[error("could not select the alias to run\n-> {0}")]
    AliasRepository(#[from] ErrorsAliasesRepository),
    #[error("could not run a command\n-> {0}")]
    SubCommand(#[from] std::io::Error),
}
