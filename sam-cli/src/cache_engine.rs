use sam_persistence::{CacheEntry, CacheError, RustBreakCache};
use sam_tui::modal_view::{ModalView, Value};
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
    DeleteEntries,
    Clear,
}

impl CacheEngine {
    pub fn run(self, cmd: CacheCommand) -> Result<i32> {
        match cmd {
            CacheCommand::PrintKeys => self.print_keys(),
            CacheCommand::Clear => self.cache_clear(),
            CacheCommand::DeleteEntries => self.delete_entries(),
        }
    }

    fn print_keys(self) -> Result<i32> {
        let cache = RustBreakCache::with_ttl(self.cache_dir, &self.ttl)?;
        println!(
            "{}{}Keys present in cache{}\n",
            termion::style::Bold,
            termion::color::Fg(termion::color::Green),
            termion::style::Reset,
        );
        for key in cache.entries()? {
            println!(
                "- {}{}{}{}",
                termion::style::Bold,
                termion::color::Fg(termion::color::Green),
                key.command,
                termion::style::Reset,
            );
        }
        Ok(0)
    }

    fn delete_entries(self) -> Result<i32> {
        let cache = RustBreakCache::with_ttl(self.cache_dir, &self.ttl)?;
        let values: Vec<CacheEntryWrapper> = cache.entries()?.map(CacheEntryWrapper).collect();
        if values.len() > 0 {
            let controller = ModalView::new(values, vec![]);
            let response = controller.run();
            if let Some(output) = response {
                for entry in output.marked_values {
                    cache.delete(&entry.0.command)?;
                }
            }
        } else {
            println!("Cache is empty. There is nothing to do!");
        }
        Ok(0)
    }

    fn cache_clear(self) -> Result<i32> {
        Ok(RustBreakCache::with_ttl(self.cache_dir, &self.ttl)?
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheEntryWrapper(CacheEntry);

impl Value for CacheEntryWrapper {
    fn text(&self) -> &str {
        &self.0.command
    }

    fn preview(&self) -> String {
        self.0.output.clone()
    }
}
