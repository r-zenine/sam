use crate::cache_engine::CacheEngine;
use crate::config::AppSettings;
use crate::config_engine::ConfigEngine;
use crate::executors::make_executor;
use crate::history_engine::HistoryEngine;
use crate::logger::{ErrorLogger, FileLogger, SilentLogger};
use crate::session_engine::SessionEngine;
use sam_core::engines::{SamEngine, SamExecutor, SamLogger, VarsDefaultValuesSetter};
use sam_persistence::repositories::{
    AliasesRepository, ErrorsAliasesRepository, ErrorsVarsRepository, VarsRepository,
};
use sam_persistence::{
    AliasHistory, CacheError, ErrorAliasHistory, NoopVarsCache, RustBreakCache, VarsCache,
    SessionError,
};
use sam_readers::read_aliases_from_path;
use sam_readers::read_vars_repository;
use sam_readers::ErrorsAliasRead;
use sam_readers::ErrorsVarRead;
use sam_tui::{ErrorsUIV2, UserInterfaceV2};
use sam_utils::fsutils;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use thiserror::Error;

pub struct Environment {
    pub aliases: AliasesRepository,
    pub vars: VarsRepository,
    pub logger: Rc<dyn SamLogger>,
    pub env_variables: HashMap<String, String>,
    pub config: AppSettings,
    pub history: AliasHistory,
    pub cache: Box<dyn VarsCache>,
}

impl Environment {
    pub fn sam_engine(
        self,
    ) -> SamEngine<UserInterfaceV2, AliasesRepository, VarsRepository, VarsRepository> {
        let executor: Rc<dyn SamExecutor> = make_executor(self.config.dry)
            .expect("Could not initialize executors, please open a ticket");
        let resolver = UserInterfaceV2::new(self.env_variables.clone(), self.cache);

        SamEngine {
            resolver,
            aliases: self.aliases,
            vars: self.vars.clone(),
            defaults: self.vars,
            logger: self.logger,
            env_variables: self.env_variables,
            history: RefCell::new(Box::new(self.history)),
            executor,
        }
    }

    pub fn cache_engine(self) -> CacheEngine {
        CacheEngine {
            cache_dir: self.config.cache_dir().to_owned(),
            ttl: self.config.ttl(),
        }
    }

    pub fn history_engine(
        self,
    ) -> HistoryEngine<UserInterfaceV2, AliasesRepository, VarsRepository, VarsRepository> {
        let history = self.history.clone();
        let sam_engine = self.sam_engine();
        HistoryEngine {
            sam_engine,
            history,
        }
    }
    // Clippy is making a false positive on this one
    #[allow(clippy::missing_const_for_fn)]
    pub fn config_engine(self) -> ConfigEngine {
        ConfigEngine {
            aliases: self.aliases,
            vars: self.vars,
            env_variables: self.env_variables,
        }
    }

    pub fn session_engine(self) -> SessionEngine {
        // Use the cache directory's parent for sessions since cache_dir is actually a file path
        let cache_parent = self.config.cache_dir().parent()
            .expect("Cache directory should have a parent")
            .to_path_buf();
        let session_file = cache_parent.join("session_storage");
        // Sessions have longer TTL than cache (24 hours vs default cache TTL)
        let session_ttl = std::time::Duration::from_secs(24 * 60 * 60);
        SessionEngine::new(session_file, session_ttl)
            .expect("Could not initialize session engine")
    }
}

pub fn from_settings(mut config: AppSettings) -> Result<Environment> {
    // Load session defaults and merge them with config defaults
    load_and_merge_session_defaults(&mut config)?;

    let cache: Box<dyn VarsCache> = if !config.no_cache {
        Box::new(RustBreakCache::with_ttl(config.cache_dir(), &config.ttl())?)
    } else {
        Box::new(NoopVarsCache {})
    };
    let history = AliasHistory::new(config.history_file(), Some(1000))?;

    let logger = logger_instance(config.silent)?;

    let mut aliases_vec = vec![];
    for f in config.aliases_files() {
        aliases_vec.extend(read_aliases_from_path(&f)?);
    }
    let aliases = AliasesRepository::new(aliases_vec.into_iter())?;

    let mut vars = VarsRepository::default();
    for f in config.vars_files() {
        vars.merge(read_vars_repository(&f)?);
    }
    vars.set_defaults(&config.defaults);
    vars.ensure_no_missing_dependency()?;

    Ok(Environment {
        aliases,
        vars,
        logger,
        env_variables: config.variables(),
        config,
        history,
        cache,
    })
}

fn load_and_merge_session_defaults(config: &mut AppSettings) -> Result<()> {
    // Create a temporary session engine to load defaults
    let cache_parent = config.cache_dir().parent()
        .expect("Cache directory should have a parent")
        .to_path_buf();
    let session_file = cache_parent.join("session_storage");
    let session_ttl = std::time::Duration::from_secs(24 * 60 * 60);
    
    // Try to load session defaults - if it fails, just continue without session defaults
    if let Ok(session_engine) = SessionEngine::new(session_file, session_ttl) {
        if let Ok(session_defaults) = session_engine.get_session_defaults() {
            config.merge_session_defaults(session_defaults);
        }
    }
    
    Ok(())
}

fn logger_instance(silent: bool) -> Result<Rc<dyn SamLogger>> {
    if !silent {
        Ok(Rc::new(FileLogger::new()))
    } else {
        Ok(Rc::new(SilentLogger))
    }
}

type Result<T> = std::result::Result<T, ErrorEnvironment>;
#[derive(Debug, Error)]
pub enum ErrorEnvironment {
    #[error("could not run the terminal user interface\n-> {0}")]
    UI(#[from] ErrorsUIV2),
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
    #[error("could not open the history file because\n-> {0}")]
    ErrAliasHistory(#[from] ErrorAliasHistory),
    #[error("could not open the vars cache because\n-> {0}")]
    CacheError(#[from] CacheError),
    #[error("could not initialize logger -> {0}")]
    LoggerError(#[from] ErrorLogger),
    #[error("could not initialize session storage -> {0}")]
    SessionError(#[from] SessionError),
}
