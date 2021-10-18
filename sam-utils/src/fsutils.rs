use rand::Rng;
use std::cell::RefCell;
use std::env::temp_dir;
use std::fs::remove_dir_all;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

type Result<T> = std::result::Result<T, ErrorsFS>;

#[derive(Debug)]
pub struct TempDirectory {
    pub path: PathBuf,
}

impl TempDirectory {
    pub fn new() -> Result<Self> {
        let seed: u16 = rand::thread_rng().gen();
        let dir_name = format!("sam-temp-dir-{}", seed);
        let path = temp_dir().join(dir_name);
        std::fs::create_dir(path.clone()).map_err(ErrorsFS::UnexpectedIOError)?;
        Ok(TempDirectory { path })
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        remove_dir_all(&self.path).expect("Can't cleanup directory")
    }
}

#[derive(Debug)]
pub struct TempFile {
    pub file: RefCell<File>,
    pub path: PathBuf,
}

impl TempFile {
    pub fn new() -> Result<TempFile> {
        let mut path = temp_dir();
        let file_name = format!("{}.tmp", Uuid::new_v4());
        path.push(file_name);

        let file = File::create(path.as_path())?;
        Ok(TempFile {
            file: RefCell::new(file),
            path,
        })
    }
}

pub fn walk_dir(path: &Path) -> Result<Vec<PathBuf>> {
    let dir_content = std::fs::read_dir(path)?;
    let paths = dir_content.flat_map(|e| e.map(|e| e.path()));
    let mut deque = vec![];
    for content in paths {
        if content.is_dir() {
            let cur_dir = std::fs::read_dir(content.as_path())?;
            let paths = cur_dir.flat_map(|e| e.map(|e| e.path()));
            deque.extend(paths);
        }
        if content.is_file() {
            deque.push(content);
        }
    }
    Ok(deque)
}

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

#[derive(Debug, Error)]
pub enum ErrorsFS {
    #[error("provided path {0} is not a directory")]
    PathNotDirectory(PathBuf),
    #[error("provided path {0} is not a file")]
    PathNotFile(PathBuf),
    #[error("provided path {0} is not exist")]
    PathDoesNotExist(PathBuf),
    #[error("insufficient permission for provided path {0}")]
    PathInsufficientPermission(PathBuf),
    #[error("got an unexpected error {0}")]
    UnexpectedIOError(#[from] std::io::Error),
}
