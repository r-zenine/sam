use crate::vars_cache::{CacheError, RocksDBVarsCache};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

pub struct CacheEngine {
    pub cache_dir: PathBuf,
    pub ttl: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CacheCommand {
    PrintKeys,
    Clear,
}

impl CacheEngine {
    pub fn run(self, cmd: CacheCommand) -> Result<i32> {
        match cmd {
            CacheCommand::PrintKeys => self.print_keys(),
            CacheCommand::Clear => self.cache_clear(),
        }
    }

    fn print_keys(self) -> Result<i32> {
        let cache = RocksDBVarsCache::new(self.cache_dir, &self.ttl);
        println!(
            "{}{}Keys present in cache{}\n",
            termion::style::Bold,
            termion::color::Fg(termion::color::Green),
            termion::style::Reset,
        );
        for key in cache.keys()? {
            println!(
                "- {}{}{}{}",
                termion::style::Bold,
                termion::color::Fg(termion::color::Green),
                key,
                termion::style::Reset,
            );
        }
        Ok(0)
    }

    fn cache_clear(self) -> Result<i32> {
        Ok(RocksDBVarsCache::new(self.cache_dir, &self.ttl)
            .clear_cache()
            .map(|_| 0)?)
    }
}

type Result<T> = std::result::Result<T, ErrorCacheEngine>;

#[derive(Debug, Error)]
pub enum ErrorCacheEngine {
    #[error("an error happened while trying to clear the cache\n -> {0}")]
    CacheClear(#[from] CacheError),
}
