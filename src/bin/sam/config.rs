use sam::utils::fsutils;
use sam::utils::fsutils::ErrorsFS;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

const CONFIG_FILE_NAME: &str = ".sam_rc.toml";

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AppSettings {
    root_dir: PathBuf,
}

type Result<T> = std::result::Result<T, ErrorsConfig>;

impl AppSettings {
    fn read_config(path: PathBuf) -> Result<config::Config> {
        let path = fsutils::ensure_exists(path)
            .and_then(fsutils::ensure_is_file)
            .and_then(fsutils::ensure_sufficient_permisions)?;
        let mut conf = config::Config::default();
        conf.merge(config::File::from(path.as_path()))
            .map_err(ErrorsConfig::from)?;
        Ok(conf)
    }

    pub fn load() -> Result<Self> {
        let home_dir_o = Self::home_dir_config_path()?;
        let current_dir_o = Self::current_dir_config_path()?;

        let config_home_dir = Self::read_config(home_dir_o);
        let config_current_dir = Self::read_config(current_dir_o);

        config_current_dir
            .or(config_home_dir)?
            .try_into::<Self>()
            .map_err(ErrorsConfig::from)
            .and_then(AppSettings::validate)
    }

    pub fn root_dir(&self) -> &'_ Path {
        self.root_dir.as_ref()
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

    fn current_dir_config_path() -> Result<PathBuf> {
        std::env::current_dir()
            .map_err(|_| ErrorsConfig::CantFindCurrentDirectory)
            .map(|e| e.join(CONFIG_FILE_NAME))
    }
}

#[derive(Debug, Error)]
pub enum ErrorsConfig {
    #[error("got the following error\n-> {0}")]
    ReadConfig(#[from] config::ConfigError),
    #[error("got the following error\n-> {0}")]
    FileSystem(#[from] ErrorsFS),
    #[error("we were unable to locate the home directory for the current user")]
    CantFindHomeDirectory,
    #[error("we were unable to locate the current directory for the current user")]
    CantFindCurrentDirectory,
}
