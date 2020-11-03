use std::fmt::Display;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, ErrorsFS>;

pub fn replace_home_variable(path: String) -> String {
    let home_dir_o = dirs::home_dir().and_then(|e| e.into_os_string().into_string().ok());
    if let Some(home_dir) = home_dir_o {
        if path.contains("$HOME") {
            return path.replace("$HOME", &home_dir);
        }
    }
    path
}

pub fn ensure_exists(path: PathBuf) -> Result<PathBuf> {
    if !path.exists() {
        Err(ErrorsFS::PathDoesNotExist(path))
    } else {
        Ok(path)
    }
}
pub fn ensure_is_directory(path: PathBuf) -> Result<PathBuf> {
    if !path.is_dir() {
        Err(ErrorsFS::PathNotDirectory(path))
    } else {
        Ok(path)
    }
}
pub fn ensure_is_file(path: PathBuf) -> Result<PathBuf> {
    if !path.is_file() {
        Err(ErrorsFS::PathNotFile(path))
    } else {
        Ok(path)
    }
}
pub fn ensure_sufficient_permisions(path: PathBuf) -> Result<PathBuf> {
    std::fs::metadata(path.as_path())
        .map_err(|_| ErrorsFS::PathInsufficientPermission(path.clone()))
        .map(|_| path)
}
#[derive(Debug)]
pub enum ErrorsFS {
    PathNotDirectory(PathBuf),
    PathNotFile(PathBuf),
    PathDoesNotExist(PathBuf),
    PathInsufficientPermission(PathBuf),
    UnexpectedIOError(std::io::Error),
}

impl Display for ErrorsFS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorsFS::PathNotDirectory(path) => {
                writeln!(f, "provided path {} is not a directory.", path.display())
            }
            ErrorsFS::PathNotFile(path) => {
                writeln!(f, "provided path {} is not a file.", path.display())
            }
            ErrorsFS::PathDoesNotExist(path) => {
                writeln!(f, "provided path {} does not exist.", path.display())
            }
            ErrorsFS::PathInsufficientPermission(path) => writeln!(
                f,
                "insufficient permission for provided path {}.",
                path.display()
            ),
            ErrorsFS::UnexpectedIOError(err) => writeln!(f, "got an unexpected error {}.", err),
        }
    }
}

impl From<std::io::Error> for ErrorsFS {
    fn from(v: std::io::Error) -> Self {
        ErrorsFS::UnexpectedIOError(v)
    }
}
