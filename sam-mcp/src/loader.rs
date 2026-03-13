use sam_config::AppSettings;
use sam_core::engines::VarsDefaultValuesSetter;
use sam_persistence::repositories::{AliasesRepository, ErrorsAliasesRepository, VarsRepository};
use sam_persistence::{CacheError, NoopVarsCache, RustBreakCache, VarsCache};
use sam_readers::{read_aliases_from_path, read_vars_repository, ErrorsAliasRead, ErrorsVarRead};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

pub struct SamContext {
    pub aliases: AliasesRepository,
    pub vars: VarsRepository,
    pub cache: Box<dyn VarsCache + Send + Sync>,
    pub env_variables: HashMap<String, String>,
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("config error: {0}")]
    Config(#[from] sam_config::ErrorsSettings),
    #[error("cache error: {0}")]
    Cache(#[from] CacheError),
    #[error("alias read error: {0}")]
    AliasRead(#[from] ErrorsAliasRead),
    #[error("var read error: {0}")]
    VarRead(#[from] ErrorsVarRead),
    #[error("alias repository error: {0}")]
    AliasRepository(#[from] ErrorsAliasesRepository),
}

pub fn load_from(path: PathBuf) -> Result<SamContext, LoadError> {
    build_context(AppSettings::load_from(path)?)
}

pub fn load() -> Result<SamContext, LoadError> {
    build_context(AppSettings::load()?)
}

fn build_context(config: AppSettings) -> Result<SamContext, LoadError> {
    let cache: Box<dyn VarsCache + Send + Sync> = if !config.no_cache {
        Box::new(RustBreakCache::with_ttl(config.cache_dir(), &config.ttl())?)
    } else {
        Box::new(NoopVarsCache {})
    };

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

    Ok(SamContext {
        aliases,
        vars,
        cache,
        env_variables: config.variables(),
    })
}
