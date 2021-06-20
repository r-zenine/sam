use sam::utils::fsutils;
use sam::utils::fsutils::ErrorsFS;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

const CONFIG_FILE_NAME: &str = ".sam_rc.toml";

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AppSettings {
    root_dir: PathBuf,
    #[serde(skip)]
    cache_dir: PathBuf,
    ttl: u64,
    #[serde(flatten)]
    variables: HashMap<String, String>,
}

type Result<T> = std::result::Result<T, ErrorsConfig>;

impl AppSettings {
    fn read_config(path: PathBuf) -> Result<AppSettings> {
        let path = fsutils::ensure_exists(path)
            .and_then(fsutils::ensure_is_file)
            .and_then(fsutils::ensure_sufficient_permisions)?;
        let content = fs::read_to_string(&path)?;
        let conf: AppSettings = toml::from_str(content.as_str())?;
        Ok(conf)
    }

    pub fn load() -> Result<Self> {
        let home_dir_o = Self::home_dir_config_path()?;
        let current_dir_o = Self::current_dir_config_path()?;

        let config_home_dir = Self::read_config(home_dir_o);
        let config_current_dir = Self::read_config(current_dir_o);

        let cache_dir = Self::cache_dir_path()?;

        config_current_dir
            .or(config_home_dir)
            .and_then(AppSettings::validate)
            .map(|mut e| {
                e.cache_dir = cache_dir;
                e
            })
    }

    pub fn root_dir(&self) -> &'_ Path {
        self.root_dir.as_ref()
    }

    pub fn ttl(&self) -> Duration {
        Duration::from_secs(self.ttl)
    }

    pub fn cache_dir(&self) -> &'_ Path {
        self.cache_dir.as_ref()
    }

    fn validate(orig: AppSettings) -> Result<AppSettings> {
        let files = fsutils::walk_dir(orig.root_dir.as_path())?;
        for f in files {
            fsutils::ensure_exists(f).and_then(fsutils::ensure_sufficient_permisions)?;
        }
        Ok(orig)
    }

    fn home_dir_config_path() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|e| e.join(CONFIG_FILE_NAME))
            .ok_or(ErrorsConfig::CantFindHomeDirectory)
    }
    fn cache_dir_path() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|e| e.join(".cache").join("sam"))
            .ok_or(ErrorsConfig::CantFindCacheDirectory)
    }

    fn current_dir_config_path() -> Result<PathBuf> {
        std::env::current_dir()
            .map_err(|_| ErrorsConfig::CantFindCurrentDirectory)
            .map(|e| e.join(CONFIG_FILE_NAME))
    }
    pub fn variables(&self) -> HashMap<String, String> {
        self.variables.clone()
    }
}

#[derive(Debug, Error)]
pub enum ErrorsConfig {
    #[error("got the following error\n-> {0}")]
    CantDeserialize(#[from] toml::de::Error),
    #[error("got the following error\n-> {0}")]
    CantReadConfigFile(#[from] io::Error),
    #[error("got the following error\n-> {0}")]
    FileSystem(#[from] ErrorsFS),
    #[error("we were unable to locate the home directory for the current user")]
    CantFindHomeDirectory,
    #[error("we were unable to locate the cache directory for the current user")]
    CantFindCacheDirectory,
    #[error("we were unable to locate the current directory for the current user")]
    CantFindCurrentDirectory,
}
