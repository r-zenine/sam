use sam::core::aliases_repository::AliasesRepository;
use sam::core::commands::unset_env_vars;
use sam::core::vars_repository::VarsRepository;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum ConfigCommand {
    CheckUnsetEnvVars,
}

pub struct ConfigEngine {
    pub aliases: AliasesRepository,
    pub vars: VarsRepository,
    pub env_variables: HashMap<String, String>,
}

impl ConfigEngine {
    pub fn run(&self, cmd: ConfigCommand) -> Result<i32> {
        match cmd {
            ConfigCommand::CheckUnsetEnvVars => self.check_unset_env_vars(),
        }
    }
    fn check_unset_env_vars(&self) -> Result<i32> {
        let missing_envvars_in_aliases = unset_env_vars(self.aliases.aliases().iter());
        let missing_envvars_in_vars = unset_env_vars(self.vars.vars_iter());
        let envvars_in_config: HashSet<&String> = self.env_variables.keys().collect();
        let all_envvars: HashSet<&String> = missing_envvars_in_vars
            .union(&missing_envvars_in_aliases)
            .collect();

        let missing_envvars: Vec<_> = all_envvars.difference(&envvars_in_config).collect();
        if missing_envvars.is_empty() {
            return Ok(0);
        }
        println!("Undifined environement variables:");
        for var in &missing_envvars {
            println!(
                "- {}{}{}{}",
                termion::style::Bold,
                termion::color::Fg(termion::color::Red),
                var,
                termion::style::Reset,
            );
        }
        Ok(1)
    }
    // TODO use conch parser to detect tools that are not available in the current machine
    // https://github.com/ipetkov/conch-parser
}

type Result<T> = std::result::Result<T, ErrorsConfigEngine>;

#[derive(Debug, Error)]
pub enum ErrorsConfigEngine {}
