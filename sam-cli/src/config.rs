use crate::cli::CLISettings;
use sam_core::choices::Choice;
use sam_core::identifiers::Identifier;
use sam_persistence::CacheError;
use sam_utils::fsutils;
use sam_utils::fsutils::walk_dir;
use sam_utils::fsutils::ErrorsFS;
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
    root_dir: Vec<PathBuf>,
    ttl: u64,
    #[serde(flatten)]
    pub env_variables: HashMap<String, String>,
    #[serde(skip)]
    cache_dir: PathBuf,
    #[serde(skip)]
    history_dir: PathBuf,
    #[serde(skip)]
    pub dry: bool,
    #[serde(skip)]
    pub silent: bool,
    #[serde(skip)]
    pub no_cache: bool,
    #[serde(skip)]
    pub defaults: HashMap<Identifier, Choice>,
}

type Result<T> = std::result::Result<T, ErrorsSettings>;

impl AppSettings {
    fn read_config(path: PathBuf) -> Result<AppSettings> {
        let path = fsutils::ensure_exists(path)
            .and_then(fsutils::ensure_is_file)
            .and_then(fsutils::ensure_sufficient_permisions)?;
        let content = fs::read_to_string(&path)?;
        let conf: AppSettings = toml::from_str(content.as_str())?;
        Ok(conf)
    }

    pub fn load(cli_settings: Option<CLISettings>) -> Result<Self> {
        let home_dir_o = Self::home_dir_config_path()?;
        let current_dir_o = Self::current_dir_config_path();

        let config_home_dir = Self::read_config(home_dir_o);
        let config_current_dir = current_dir_o.and_then(Self::read_config);

        let cache_dir = Self::cache_dir_path()?;
        let history_dir = Self::history_dir_path()?;

        let mut settings = config_current_dir
            .or(config_home_dir)
            .and_then(AppSettings::validate)
            .map(|mut e| {
                e.cache_dir = cache_dir;
                e.history_dir = history_dir;
                e
            })?;

        if let Some(m) = cli_settings {
            settings.merge_command_line_args(m);
        }

        Ok(settings)
    }

    fn merge_command_line_args(&mut self, cmd_args: CLISettings) {
        self.dry = cmd_args.dry;
        self.silent = cmd_args.silent;
        self.no_cache = cmd_args.no_cache;
        self.defaults = cmd_args.default_choices.0;
    }

    pub fn ttl(&self) -> Duration {
        Duration::from_secs(self.ttl)
    }

    pub fn cache_dir(&self) -> &'_ Path {
        self.cache_dir.as_ref()
    }

    pub fn history_dir(&self) -> &'_ Path {
        self.history_dir.as_ref()
    }

    fn validate(orig: AppSettings) -> Result<AppSettings> {
        for path in &orig.root_dir {
            if let Ok(files) = fsutils::walk_dir(path) {
                for f in files {
                    fsutils::ensure_exists(f).and_then(fsutils::ensure_sufficient_permisions)?;
                }
            }
        }
        Ok(orig)
    }

    fn home_dir_config_path() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|e| e.join(CONFIG_FILE_NAME))
            .ok_or(ErrorsSettings::CantFindHomeDirectory)
    }

    fn cache_dir_path() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|e| e.join(".cache").join("sam"))
            .ok_or(ErrorsSettings::CantFindCacheDirectory)
    }

    fn history_dir_path() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|e| e.join(".local").join("share").join("sam"))
            .ok_or(ErrorsSettings::CantFindCacheDirectory)
    }

    fn current_dir_config_path() -> Result<PathBuf> {
        std::env::current_dir()
            .map_err(|_| ErrorsSettings::CantFindCurrentDirectory)
            .map(|e| e.join(CONFIG_FILE_NAME))
    }
    pub fn variables(&self) -> HashMap<String, String> {
        self.env_variables.clone()
    }

    fn sam_files(&self) -> impl Iterator<Item = PathBuf> + '_ {
        self.root_dir
            .iter()
            .map(AsRef::as_ref)
            .flat_map(walk_dir)
            .flatten()
    }

    pub fn aliases_files(&self) -> impl Iterator<Item = PathBuf> + '_ {
        self.sam_files().filter(|f| {
            if let Some(file_name) = f.file_name() {
                file_name == "aliases.yaml" || file_name == "aliases.yml"
            } else {
                false
            }
        })
    }

    pub fn vars_files(&self) -> impl Iterator<Item = PathBuf> + '_ {
        self.sam_files().filter(|f| {
            if let Some(file_name) = f.file_name() {
                file_name == "vars.yaml" || file_name == "vars.yml"
            } else {
                false
            }
        })
    }
}

#[derive(Debug, Error)]
pub enum ErrorsSettings {
    #[error("got deserialize the configuration file because\n-> {0}")]
    CantDeserialize(#[from] toml::de::Error),
    #[error("can't read the configuration file because\n-> {0}")]
    CantReadConfigFile(#[from] io::Error),
    #[error("got the following file-system related error\n-> {0}")]
    FileSystem(#[from] ErrorsFS),
    #[error("could not initialize the cache\n-> {0}")]
    VarsCache(#[from] CacheError),
    #[error("we were unable to locate the home directory for the current user")]
    CantFindHomeDirectory,
    #[error("we were unable to locate the cache directory for the current user")]
    CantFindCacheDirectory,
    #[error("we were unable to locate the current directory for the current user")]
    CantFindCurrentDirectory,
}
