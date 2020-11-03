use serde::{Deserialize, Serialize};
use ssam::utils::fsutils;
use ssam::utils::fsutils::ErrorsFS;
use std::fmt::Display;
use std::path::{Path, PathBuf};
// Todo
// 1. get rid of RawAppSettings.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AppSettings {
    scripts_dir: PathBuf,
    aliases_file: PathBuf,
    vars_file: PathBuf,
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
            .map(|e| e.join(".ssam_rc.toml"))
            .ok_or(ErrorsConfig::ErrorCantFindHomeDirectory)?;
        let current_dir_o = std::env::current_dir()
            .map_err(|_| ErrorsConfig::ErrorCantFindCurrentDirectory)
            .map(|e| e.join("ssam_rc.toml"))?;

        let config_home_dir = Self::load_from_path(home_dir_o);
        let config_current_dir = Self::load_from_path(current_dir_o);
        config_home_dir
            .or(config_current_dir)?
            .try_into::<Self>()
            .map_err(ErrorsConfig::from)
            .and_then(AppSettings::validate)
    }

    pub fn scripts_dir(&self) -> &'_ Path {
        self.scripts_dir.as_ref()
    }
    pub fn aliases_file(&self) -> &'_ Path {
        self.aliases_file.as_ref()
    }
    pub fn vars_file(&self) -> &'_ Path {
        self.vars_file.as_ref()
    }

    fn validate(orig: AppSettings) -> Result<AppSettings> {
        let mut s = orig.clone();

        s.aliases_file = fsutils::ensure_exists(s.aliases_file)
            .and_then(fsutils::ensure_is_file)
            .and_then(fsutils::ensure_sufficient_permisions)?;

        s.vars_file = fsutils::ensure_exists(s.vars_file)
            .and_then(fsutils::ensure_is_file)
            .and_then(fsutils::ensure_sufficient_permisions)?;

        s.scripts_dir = fsutils::ensure_exists(s.scripts_dir.clone())
            .and_then(fsutils::ensure_is_directory)
            .and_then(fsutils::ensure_sufficient_permisions)?;

        Ok(s.to_owned())
    }
}

#[derive(Debug)]
pub enum ErrorsConfig {
    ErrorReadConfig(config::ConfigError),
    ErrorFS(ErrorsFS),
    ErrorCantFindHomeDirectory,
    ErrorCantFindCurrentDirectory,
}

impl Display for ErrorsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "while loading and validating the configuration, ")?;
        match self {
            ErrorsConfig::ErrorReadConfig(erc) => {
                writeln!(f, "got the following error: \n -> {}", erc)
            }
            ErrorsConfig::ErrorCantFindHomeDirectory => writeln!(
                f,
                "we were unable to locate the home directory for the current user."
            ),
            ErrorsConfig::ErrorFS(fs_error) => {
                writeln!(f, "got the following error: \n -> {}", fs_error)
            }
            ErrorsConfig::ErrorCantFindCurrentDirectory => writeln!(
                f,
                "we were unable to locate the current directory for the current user."
            ),
        }
    }
}

impl From<ErrorsFS> for ErrorsConfig {
    fn from(v: ErrorsFS) -> Self {
        ErrorsConfig::ErrorFS(v)
    }
}

impl From<config::ConfigError> for ErrorsConfig {
    fn from(v: config::ConfigError) -> Self {
        ErrorsConfig::ErrorReadConfig(v)
    }
}
