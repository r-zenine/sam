use serde::{Deserialize, Serialize};
use ssam::utils::fsutils;
use ssam::utils::fsutils::ErrorsFS;
use std::path::{Path, PathBuf};
use thiserror::Error;
// Todo
// 1. get rid of RawAppSettings.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AppSettings {
    root_dir: PathBuf,
}

type Result<T> = std::result::Result<T, ErrorsConfig>;

impl AppSettings {
    fn load_from_path(path: PathBuf) -> Result<config::Config> {
        let path = fsutils::ensure_exists(path)
            .and_then(fsutils::ensure_is_file)
            .and_then(fsutils::ensure_sufficient_permisions)?;
        let mut conf = config::Config::default();
        conf.merge(config::File::from(path.as_path()))
            .map_err(ErrorsConfig::from)?;
        Ok(conf)
    }

    pub fn load() -> Result<Self> {
        let home_dir_o = dirs::home_dir()
            .map(|e| e.join(".sam_rc.toml"))
            .ok_or(ErrorsConfig::CantFindHomeDirectory)?;
        let current_dir_o = std::env::current_dir()
            .map_err(|_| ErrorsConfig::CantFindCurrentDirectory)
            .map(|e| e.join("sam_rc.toml"))?;

        let config_home_dir = Self::load_from_path(home_dir_o);
        let config_current_dir = Self::load_from_path(current_dir_o);
        config_home_dir
            .or(config_current_dir)?
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
