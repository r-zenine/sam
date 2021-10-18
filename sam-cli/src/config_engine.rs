use sam_core::commands::programs_used;
use sam_core::commands::unset_env_vars;
use sam_core::repositories::AliasesRepository;
use sam_core::repositories::VarsRepository;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigCommand {
    #[allow(dead_code)]
    CheckUnsetEnvVars,
    #[allow(dead_code)]
    CheckUnavailablePrograms,
    All,
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
            ConfigCommand::CheckUnavailablePrograms => self.check_unavailable_programs(),
            ConfigCommand::All => {
                self.check_unavailable_programs()?;
                self.check_unset_env_vars()
            }
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

    fn check_unavailable_programs(&self) -> Result<i32> {
        let programs_in_aliases = programs_used(self.aliases.aliases().iter());
        let programs_in_vars = programs_used(self.vars.vars_iter());
        let mut missing_programs = vec![];
        for prg in programs_in_aliases.union(&programs_in_vars) {
            if !Self::is_program_available(prg) {
                missing_programs.push(prg)
            }
        }
        if !missing_programs.is_empty() {
            println!("Missing programs:");
            for prg in missing_programs {
                println!(
                    "- {}{}{}{}",
                    termion::style::Bold,
                    termion::color::Fg(termion::color::Red),
                    prg,
                    termion::style::Reset,
                );
            }
        }
        Ok(1)
    }

    fn is_program_available(program: &str) -> bool {
        if let Ok(cmd) = std::process::Command::new("which").arg(program).output() {
            cmd.status.success()
        } else {
            false
        }
    }
}

type Result<T> = std::result::Result<T, ErrorsConfigEngine>;

#[derive(Debug, Error)]
pub enum ErrorsConfigEngine {}
