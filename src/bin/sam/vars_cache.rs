use rocksdb::DB;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTimeError;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

pub trait VarsCache {
    fn put(&self, command: &dyn AsRef<str>, output: &dyn AsRef<str>) -> Result<(), CacheError>;
    fn get(&self, command: &dyn AsRef<str>) -> Result<Option<String>, CacheError>;
}

pub struct NoopVarsCache {}

impl VarsCache for NoopVarsCache {
    fn put(&self, _command: &dyn AsRef<str>, _output: &dyn AsRef<str>) -> Result<(), CacheError> {
        Ok(())
    }
    fn get(&self, _command: &dyn AsRef<str>) -> Result<Option<String>, CacheError> {
        Ok(None)
    }
}

#[derive(Debug)]
pub struct RocksDBVarsCache {
    path: PathBuf,
    ttl: Duration,
}

impl RocksDBVarsCache {
    // TODO expose a version without the TTL
    pub fn new(p: impl AsRef<Path>, ttl: &Duration) -> Self {
        RocksDBVarsCache {
            path: p.as_ref().to_owned(),
            ttl: *ttl,
        }
    }
    pub fn open_cache(&self) -> Result<DB, CacheError> {
        let mut options = rocksdb::Options::default();
        options.create_if_missing(true);
        DB::open_with_ttl(&options, self.path.clone(), self.ttl)
            .map_err(|e| CacheError::RocksDBOpenError(self.path.clone(), e))
    }

    pub fn invalidate_if_too_old(&self, c: Option<CacheEntry>) -> Option<CacheEntry> {
        c.filter(|e| e.is_valid(self.ttl))
    }

    pub fn clear_cache(&self) -> Result<(), CacheError> {
        let db = self.open_cache()?;
        let keys = db.iterator(rocksdb::IteratorMode::Start);
        for (key, _) in keys {
            db.delete(key).map_err(CacheError::RocksDBError)?;
        }
        Ok(())
    }

    pub fn keys(&self) -> Result<Vec<String>, CacheError> {
        let db = self.open_cache()?;
        let keys = db.iterator(rocksdb::IteratorMode::Start);
        Ok(keys
            .into_iter()
            .map(|(key, _)| String::from_utf8_lossy(key.as_ref()).to_string())
            .collect())
    }
}

impl VarsCache for RocksDBVarsCache {
    fn put(&self, command: &dyn AsRef<str>, output: &dyn AsRef<str>) -> Result<(), CacheError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;

        let v = CacheEntry {
            output: output.as_ref().to_string(),
            creation_date: now.as_secs(),
        };

        let bytes = bincode::serialize(&v)?;
        let db = self.open_cache()?;

        db.put(command.as_ref(), bytes)
            .map_err(CacheError::RocksDBError)?;
        db.flush().map_err(CacheError::RocksDBError)
    }

    fn get(&self, command: &dyn AsRef<str>) -> Result<Option<String>, CacheError> {
        self.open_cache()?
            .get(command.as_ref())?
            .as_deref()
            .map(bincode::deserialize::<CacheEntry>)
            .transpose()
            .map_err(CacheError::CacheEntryDeserializationErr)
            .map(|e| self.invalidate_if_too_old(e).map(|e| e.output))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    creation_date: u64,
    output: String,
}

impl CacheEntry {
    pub fn is_valid(&self, ttl: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("can't get timestamp from OS")
            .as_secs();
        (now - ttl.as_secs()) < self.creation_date
    }
}
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("can't open rockdb at path {0} because\n-> {1}")]
    RocksDBOpenError(PathBuf, rocksdb::Error),
    #[error("can't interract with rocksdb because\n-> {0}")]
    RocksDBError(#[from] rocksdb::Error),
    #[error("can't serialize value for cache insertion because\n-> {0}")]
    CacheEntrySerializationErr(#[from] bincode::Error),
    #[error("can't deserialize value for cache insertion because\n-> {0}")]
    CacheEntryDeserializationErr(bincode::Error),
    #[error("could not get a timestamp from the system because\n-> {0}")]
    CantGetTimeStamp(#[from] SystemTimeError),
}

#[cfg(test)]
mod tests {
    use super::RocksDBVarsCache;
    use crate::vars_cache::VarsCache;
    use sam::utils::fsutils::TempDirectory;
    use std::time::Duration;

    #[test]
    pub fn test_rocksdb_cache() {
        let tmp_dir = TempDirectory::new().expect("can't create a temporary directory");
        let ttl = Duration::from_secs(90);
        let cache = RocksDBVarsCache::new(&tmp_dir.path, &ttl);
        cache
            .put(&String::from("command"), &String::from("output"))
            .expect("can't write in rocksdb cache");
        let value = cache
            .get(&String::from("command"))
            .expect("can't read from rocksdb cache")
            .expect("can't retrieve the value from rocksdb cache");
        assert_eq!(value, "output");
    }
}
