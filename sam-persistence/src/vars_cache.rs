use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::time::Duration;
use std::time::SystemTimeError;
use thiserror::Error;

use crate::associative_state::AssociativeStateWithTTL;
use crate::associative_state::ErrorAssociativeState;

pub trait VarsCache {
    fn put(&self, command: &dyn AsRef<str>, output: &dyn AsRef<str>) -> Result<(), CacheError>;
    fn get(&self, command: &dyn AsRef<str>) -> Result<Option<String>, CacheError>;
}

#[derive(Debug)]
pub struct RustBreakCache {
    state: AssociativeStateWithTTL<CacheEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct CacheEntry {
    pub command: String,
    pub output: String,
}

impl RustBreakCache {
    pub fn with_ttl(p: impl AsRef<Path>, ttl: &Duration) -> Result<Self, CacheError> {
        Ok(RustBreakCache {
            state: AssociativeStateWithTTL::<CacheEntry>::with_ttl(p, ttl)?,
        })
    }

    pub fn entries(&self) -> Result<impl Iterator<Item = CacheEntry>, CacheError> {
        Ok(self.state.entries()?.map(|(_, v)| v))
    }

    pub fn delete(&self, key: &str) -> Result<Option<CacheEntry>, CacheError> {
        Ok(self.state.delete(key)?)
    }

    pub fn clear_cache(&self) -> Result<(), CacheError> {
        for (key, _) in self.state.entries()? {
            self.state.delete(key)?;
        }
        Ok(())
    }
}

impl VarsCache for RustBreakCache {
    fn put(&self, command: &dyn AsRef<str>, output: &dyn AsRef<str>) -> Result<(), CacheError> {
        let key = command.as_ref().to_string();
        let entry = CacheEntry {
            command: key.clone(),
            output: output.as_ref().to_string(),
        };
        Ok(self.state.put(key, entry)?)
    }

    fn get(&self, command: &dyn AsRef<str>) -> Result<Option<String>, CacheError> {
        let cache_key = command.as_ref();
        Ok(self.state.get(cache_key)?.map(|v| v.output))
    }
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

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("can't interract with rustbreak because\n-> {0}")]
    RustbreakError(#[from] rustbreak::RustbreakError),
    #[error("could not get a timestamp from the system because\n-> {0}")]
    CantGetTimeStamp(#[from] SystemTimeError),
    #[error("could not interract with cache because\n-> {0}")]
    ErrAssociativeState(#[from] ErrorAssociativeState),
}

#[cfg(test)]
mod tests {
    use crate::vars_cache::{RustBreakCache, VarsCache};
    use sam_utils::fsutils::TempFile;
    use std::time::Duration;

    #[test]
    pub fn test_rustbreak_cache() {
        let tmp_dir = TempFile::new().expect("can't create a temporary file");
        let ttl = Duration::from_secs(90);
        let cache = RustBreakCache::with_ttl(&tmp_dir.path, &ttl).expect("Can't open cache");
        cache
            .put(&String::from("command"), &String::from("output"))
            .expect("can't write in rustbreak cache");

        let cache2 = RustBreakCache::with_ttl(&tmp_dir.path, &ttl).expect("Can't open cache");
        let value = cache2
            .get(&String::from("command"))
            .expect("can't read from rustbreak cache")
            .expect("can't retrieve the value from rustbreak cache");
        assert_eq!(value, "output");

        let cache = RustBreakCache::with_ttl(&tmp_dir.path, &ttl).expect("Can't open cache");
        cache
            .put(&String::from("command2"), &String::from("output"))
            .expect("can't write in rustbreak cache");

        let value = cache2
            .get(&String::from("command2"))
            .expect("can't read from rustbreak cache")
            .expect("can't retrieve the value from rustbreak cache");
        assert_eq!(value, "output");
    }
}
