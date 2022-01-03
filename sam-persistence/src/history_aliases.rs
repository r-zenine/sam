use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

use sam_core::{
    engines::{ErrorSamEngine, SamHistory},
    entities::aliases::ResolvedAlias,
};

use crate::sequential_state::{ErrorSequentialState, SequentialState};

#[derive()]
pub struct AliasHistory {
    state: SequentialState<HistoryEntry>,
    pwd: PathBuf,
}

#[derive(Debug, Error)]
pub enum ErrorAliasHistory {
    #[error("failed to interact with alias history\n->{0}")]
    ErrSequentialState(#[from] ErrorSequentialState),
}

impl AliasHistory {
    pub fn new(
        path: impl Into<PathBuf>,
        max_size: Option<usize>,
    ) -> Result<Self, ErrorAliasHistory> {
        let state = SequentialState::new(path.into(), max_size)?;
        let pwd = std::env::current_dir().expect("can't figure out local directory");
        Ok(AliasHistory { state, pwd })
    }
}

impl SamHistory for AliasHistory {
    fn put(&mut self, alias: ResolvedAlias) -> Result<(), ErrorSamEngine> {
        let entry = HistoryEntry {
            r: alias,
            pwd: self.pwd.to_string_lossy().to_string(),
        };
        self.state
            .push(entry)
            .map_err(|err| ErrorSamEngine::HistoryNotAvailable(Box::new(err)))
    }

    fn get_last_n(&self, n: usize) -> Result<Vec<ResolvedAlias>, ErrorSamEngine> {
        let entries = self
            .state
            .entries()
            .map_err(|err| ErrorSamEngine::HistoryNotAvailable(Box::new(err)))?;
        let entries_vec: Vec<ResolvedAlias> = entries.map(|e| e.r).collect();
        if entries_vec.len() > n {
            let skip = entries_vec.len() - n;
            Ok(entries_vec.into_iter().skip(skip).collect())
        } else {
            Ok(entries_vec)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct HistoryEntry {
    r: ResolvedAlias,
    pwd: String,
}

#[cfg(test)]
mod tests {
    use sam_core::{
        engines::SamHistory,
        entities::{aliases::ResolvedAlias, choices::Choice, identifiers::Identifier},
    };
    use sam_utils::fsutils;

    use super::AliasHistory;

    #[test]
    fn test_history_put() {
        let f = fsutils::TempFile::new().expect("can't create temp file for test");
        let mut hist = AliasHistory::new(f.path, None).expect("can't create history file");
        let test = ResolvedAlias::new(
            Identifier::with_namespace("alias", Some("ns")),
            String::from("desc"),
            String::from("echo {{var}}"),
            String::from("echo choice"),
            maplit::hashmap! {
                Identifier::new("var") => Choice::new("choice", None),
            },
        );
        hist.put(test.clone()).expect("The put should succeed");
        let last = hist
            .get_last()
            .expect("should be able to read")
            .expect("Expecting a value to be returned");
        assert_eq!(test, last);
    }

    #[test]
    fn test_history_get_last_n() {}
}
