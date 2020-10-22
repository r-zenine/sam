use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::fmt::Display;
use std::path::{Path, PathBuf};
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AppSettings {
    scripts_dir: PathBuf,
    aliases_file: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct RawAppSettings {
    scripts_dir: String,
    aliases_file: String,
}

type Result<T> = std::result::Result<T, ConfigError>;

impl RawAppSettings {
    pub fn load() -> Result<RawAppSettings> {
        let home_dir_o = dirs::home_dir().map(|e| e.join(".ssam_rc.toml"));
        let mut initial_config = config::Config::default();
        let mut settings_r = Ok(&mut initial_config);
        if let Ok(_) = std::fs::metadata("ssam_rc.toml") {
            settings_r =
                settings_r.and_then(|conf| conf.merge(config::File::with_name("ssam_rc.toml")));
        }
        if let Some(home_dir) = home_dir_o {
            if home_dir.exists() {
                settings_r = settings_r.and_then(|conf| conf.merge(config::File::from(home_dir)));
            }
        }
        settings_r?
            .to_owned()
            .try_into::<RawAppSettings>()
            .map_err(|op| op.into())
    }
}

impl AppSettings {
    pub fn load() -> Result<Self> {
        RawAppSettings::load().and_then(|op| op.try_into())
    }

    pub fn scripts_dir(&self) -> &'_ Path {
        self.scripts_dir.as_ref()
    }
    pub fn aliases_file(&self) -> &'_ Path {
        self.aliases_file.as_ref()
    }
}

impl TryFrom<RawAppSettings> for AppSettings {
    type Error = self::ConfigError;

    fn try_from(value: RawAppSettings) -> std::result::Result<Self, Self::Error> {
        let mut settings = AppSettings::default();
        let aliases_path = Path::new(&value.aliases_file);
        let scripts_path = Path::new(&value.scripts_dir);

        if !(std::fs::metadata(aliases_path)?.is_file()) {
            return Err(ConfigError::ErrorPathNotFile(aliases_path.to_owned()));
        } else {
            settings.aliases_file = aliases_path.to_owned()
        }
        if !(std::fs::metadata(scripts_path)?.is_dir()) {
            return Err(ConfigError::ErrorPathNotDirectory(scripts_path.to_owned()));
        } else {
            settings.scripts_dir = scripts_path.to_owned()
        }

        Ok(settings)
    }
}

#[derive(Debug)]
pub enum ConfigError {
    ErrorReadConfig(config::ConfigError),
    ErrorPathNotDirectory(PathBuf),
    ErrorPathNotFile(PathBuf),
    ErrorPathInsufficientPermission(std::io::Error),
    ErrorPathDoesNotExist(std::io::Error),
    ErrorUnexpectedIOError(std::io::Error),
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ErrorReadConfig(erc) => {
                writeln!(f, "reading the configuration failed because {}", erc)
            }
            ConfigError::ErrorPathNotDirectory(p) => writeln!(
                f,
                "configuration invalid: the provided path {} should be a directory.",
                p.display()
            ),
            ConfigError::ErrorPathNotFile(p) => writeln!(
                f,
                "configuration invalid: the provided path {} should be a file.",
                p.display()
            ),
            ConfigError::ErrorPathInsufficientPermission(e) => {
                writeln!(f, "configuration invalid: missing permissions {}", e)
            }
            ConfigError::ErrorPathDoesNotExist(e) => {
                writeln!(f, "configuration invalid: path does not exist {}", e)
            }
            ConfigError::ErrorUnexpectedIOError(e) => writeln!(
                f,
                "configuration invalid: an expected io error happened {}",
                e
            ),
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(v: std::io::Error) -> Self {
        match v.kind() {
            std::io::ErrorKind::NotFound => ConfigError::ErrorPathDoesNotExist(v),
            std::io::ErrorKind::PermissionDenied => ConfigError::ErrorPathInsufficientPermission(v),
            std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::NotConnected
            | std::io::ErrorKind::AddrInUse
            | std::io::ErrorKind::AddrNotAvailable
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::AlreadyExists
            | std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::InvalidInput
            | std::io::ErrorKind::InvalidData
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::WriteZero
            | std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::Other
            | std::io::ErrorKind::UnexpectedEof
            | _ => ConfigError::ErrorUnexpectedIOError(v),
        }
    }
}

impl From<config::ConfigError> for ConfigError {
    fn from(v: config::ConfigError) -> Self {
        ConfigError::ErrorReadConfig(v)
    }
}
