use rocksdb::WriteBatch;
use rocksdb::WriteOptions;
use rocksdb::DB;
use sam_core::engines::ErrorSamEngine;
use sam_core::entities::aliases::ResolvedAlias;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTimeError;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

use sam_core::engines::SamHistory;

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
pub struct RocksDBCache {
    path: PathBuf,
    ttl: Option<Duration>,
}

impl RocksDBCache {
    // TODO expose a version without the TTL
    pub fn with_ttl(p: impl AsRef<Path>, ttl: &Duration) -> Self {
        RocksDBCache {
            path: p.as_ref().to_owned(),
            ttl: Some(*ttl),
        }
    }

    pub fn new(p: impl AsRef<Path>) -> Self {
        RocksDBCache {
            path: p.as_ref().to_owned(),
            ttl: None,
        }
    }

    pub fn open_cache(&self) -> Result<DB, CacheError> {
        let mut options = rocksdb::Options::default();
        options.create_if_missing(true);
        if let Some(ttl) = self.ttl {
            DB::open_with_ttl(&options, self.path.clone(), ttl)
                .map_err(|e| CacheError::RocksDBOpenError(self.path.clone(), e))
        } else {
            DB::open(&options, self.path.clone())
                .map_err(|e| CacheError::RocksDBOpenError(self.path.clone(), e))
        }
    }

    pub fn invalidate_if_too_old(&self, c: Option<CacheEntry>) -> Option<CacheEntry> {
        c.filter(|e| e.is_valid(self.ttl))
    }

    pub fn clear_cache(&self) -> Result<(), CacheError> {
        let db = self.open_cache()?;
        let keys = db.iterator(rocksdb::IteratorMode::Start);
        let mut batch = WriteBatch::default();
        for (key, _) in keys {
            batch.delete(key);
        }
        let mut options = WriteOptions::default();
        options.set_sync(true);

        db.write_opt(batch, &options)
            .map_err(CacheError::RocksDBError)
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

impl VarsCache for RocksDBCache {
    fn put(&self, command: &dyn AsRef<str>, output: &dyn AsRef<str>) -> Result<(), CacheError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;

        let v = CacheEntry {
            output: output.as_ref().to_string(),
            creation_date: now.as_secs(),
        };

        let bytes = bincode::serialize(&v)?;
        let db = self.open_cache()?;

        let mut options = WriteOptions::default();
        options.set_sync(true);

        db.put_opt(command.as_ref(), bytes, &options)
            .map_err(CacheError::RocksDBError)
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

impl SamHistory for RocksDBCache {
    fn get_last_n(&self, n: usize) -> std::result::Result<Vec<ResolvedAlias>, ErrorSamEngine> {
        let db = self
            .open_cache()
            .map_err(|err| ErrorSamEngine::HistoryNotAvailable(Box::new(err)))?;
        let keys = db.iterator(rocksdb::IteratorMode::End);
        Ok(keys
            .into_iter()
            .take(n)
            .flat_map(|(_, value)| serde_yaml::from_slice(&value))
            .collect())
    }

    fn put(&self, alias: ResolvedAlias) -> std::result::Result<(), ErrorSamEngine> {
        let db = self
            .open_cache()
            .map_err(|err| ErrorSamEngine::HistoryNotAvailable(Box::new(err)))?;
        let key = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|err| ErrorSamEngine::HistoryNotAvailable(Box::new(err)))?
            .as_secs();
        let bin_ts = key.to_le_bytes();
        let alias_bytes = serde_yaml::to_vec(&alias)
            .map_err(|err| ErrorSamEngine::HistoryNotAvailable(Box::new(err)))?;

        let mut options = WriteOptions::default();
        options.set_sync(true);
        db.put_opt(bin_ts, alias_bytes, &options)
            .map_err(|err| ErrorSamEngine::HistoryNotAvailable(Box::new(err)))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    creation_date: u64,
    output: String,
}

impl CacheEntry {
    pub fn is_valid(&self, ttl: Option<Duration>) -> bool {
        if let Some(t) = ttl {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("can't get timestamp from OS")
                .as_secs();
            (now - t.as_secs()) < self.creation_date
        } else {
            true
        }
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
    use super::RocksDBCache;
    use crate::vars_cache::VarsCache;
    use sam_utils::fsutils::TempDirectory;
    use std::time::Duration;

    #[test]
    pub fn test_rocksdb_cache() {
        let tmp_dir = TempDirectory::new().expect("can't create a temporary directory");
        let ttl = Duration::from_secs(90);
        let cache = RocksDBCache::with_ttl(&tmp_dir.path, &ttl);
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
