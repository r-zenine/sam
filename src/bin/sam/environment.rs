use crate::cache_engine::CacheEngine;
use crate::config::AppSettings;
use crate::config_engine::ConfigEngine;
use crate::executors::{DryExecutor, ShellExecutor};
use crate::logger::{SilentLogger, StdErrLogger};
use crate::sam_engine::{SamEngine, SamExecutor, SamHistory, SamLogger};
use crate::userinterface::ErrorsUI;
use crate::userinterface::UserInterface;
use crate::vars_cache::{NoopVarsCache, RocksDBCache, VarsCache};
use sam::core::aliases_repository::AliasesRepository;
use sam::core::aliases_repository::ErrorsAliasesRepository;
use sam::core::vars_repository::ErrorsVarsRepository;
use sam::core::vars_repository::VarsRepository;
use sam::io::readers::read_aliases_from_path;
use sam::io::readers::read_vars_repository;
use sam::io::readers::ErrorsAliasRead;
use sam::io::readers::ErrorsVarRead;
use sam::utils::fsutils;
use sam::utils::fsutils::walk_dir;
use std::collections::HashMap;
use std::rc::Rc;
use thiserror::Error;

pub struct Environment {
    // TODO Todo remove user interface from the context
    pub ui_interface: UserInterface,
    pub aliases: AliasesRepository,
    pub vars: VarsRepository,
    pub logger: Rc<dyn SamLogger>,
    pub env_variables: HashMap<String, String>,
    pub config: AppSettings,
    pub history: Box<dyn SamHistory>,
}

impl Environment {
    pub fn sam_engine(self) -> SamEngine<UserInterface> {
        let executor: Rc<dyn SamExecutor> = if self.config.dry {
            Rc::new(DryExecutor {})
        } else {
            Rc::new(ShellExecutor {})
        };

        SamEngine {
            resolver: self.ui_interface,
            aliases: self.aliases,
            vars: self.vars,
            logger: self.logger,
            env_variables: self.env_variables,
            history: self.history,
            executor,
        }
    }

    pub fn cache_engine(self) -> CacheEngine {
        CacheEngine {
            cache_dir: self.config.cache_dir().to_owned(),
            ttl: self.config.ttl(),
        }
    }

    pub fn config_engine(self) -> ConfigEngine {
        ConfigEngine {
            aliases: self.aliases,
            vars: self.vars,
            env_variables: self.env_variables,
        }
    }
}

pub fn from_settings(config: AppSettings) -> Result<Environment> {
    let cache: Box<dyn VarsCache> = if !config.no_cache {
        Box::new(RocksDBCache::with_ttl(config.cache_dir(), &config.ttl()))
    } else {
        Box::new(NoopVarsCache {})
    };
    let history: Box<dyn SamHistory> = Box::new(RocksDBCache::new(config.history_dir()));

    let logger = logger_instance(config.silent);
    let ui_interface = UserInterface::new(config.variables(), cache)?;
    let files = walk_dir(config.root_dir())?;
    let mut aliases_vec = vec![];
    let mut vars = VarsRepository::default();
    for f in files {
        if let Some(file_name) = f.file_name() {
            if file_name == "aliases.yaml" || file_name == "aliases.yml" {
                aliases_vec.extend(read_aliases_from_path(f.as_path())?);
            } else if file_name == "vars.yaml" || file_name == "vars.yml" {
                vars.merge(read_vars_repository(f.as_path())?);
            }
        }
    }
    vars.set_defaults(&config.defaults)?;
    let aliases = AliasesRepository::new(aliases_vec.into_iter())?;
    vars.ensure_no_missing_dependency()?;
    Ok(Environment {
        ui_interface,
        aliases,
        vars,
        logger,
        env_variables: config.variables(),
        config,
        history,
    })
}

fn logger_instance(silent: bool) -> Rc<dyn SamLogger> {
    if !silent {
        Rc::new(StdErrLogger)
    } else {
        Rc::new(SilentLogger)
    }
}

type Result<T> = std::result::Result<T, ErrorEnvironment>;
#[derive(Debug, Error)]
pub enum ErrorEnvironment {
    #[error("could not run the terminal user interface\n-> {0}")]
    UI(#[from] ErrorsUI),
    #[error("filesystem related error\n-> {0}")]
    FilesLookup(#[from] fsutils::ErrorsFS),
    #[error("could not read aliases\n-> {0}")]
    AliasRead(#[from] ErrorsAliasRead),
    #[error("could not read vars\n-> {0}")]
    VarRead(#[from] ErrorsVarRead),
    #[error("could not figure out dependencies\n-> {0}")]
    VarsRepository(#[from] ErrorsVarsRepository),
    #[error("could not figure out alias substitution\n-> {0}")]
    AliasRepository(#[from] ErrorsAliasesRepository),
}
