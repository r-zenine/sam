use rustbreak::deser::Ron;
use rustbreak::FileDatabase;
use rustbreak::RustbreakError;
use std::marker::PhantomData;
use std::path::Path;
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct SequentialState<V> {
    path: PathBuf,
    max_size: Option<usize>,
    _marker: PhantomData<V>,
}

#[derive(Error, Debug)]
pub enum ErrorSequentialState {
    #[error("failed to create sequential state because\n->{0}")]
    CreationFailure(RustbreakError),
    #[error("failed to initialize sequential state because\n->{0}")]
    InitFailure(RustbreakError),
    #[error("failed to load sequential state because\n->{0}")]
    OpenFailure(RustbreakError),
    #[error("failed to write to sequential state because\n->{0}")]
    WriteFailures(RustbreakError),
    #[error("failed to save to sequential state because\n->{0}")]
    SaveFailures(RustbreakError),
    #[error("failed to read from sequential state because\n->{0}")]
    ReadFailure(RustbreakError),
}

pub type ModResult<V> = std::result::Result<V, ErrorSequentialState>;

type Fdb<V> = FileDatabase<Vec<V>, Ron>;

pub trait Value: Serialize + DeserializeOwned + Send + Clone + std::fmt::Debug {}
impl<T> Value for T where T: Serialize + DeserializeOwned + Send + Clone + std::fmt::Debug {}

impl<V> SequentialState<V>
where
    V: Value,
{
    pub fn new(p: impl AsRef<Path>, max_size: Option<usize>) -> ModResult<Self> {
        let db = SequentialState {
            path: p.as_ref().to_owned(),
            max_size,
            _marker: PhantomData::default(),
        };
        db.open_db()?;
        Ok(db)
    }

    pub fn push(&self, entry: V) -> ModResult<()> {
        let db = self.open_db()?;
        db.write(|db| {
            db.push(entry);
            if let Some(max_size) = self.max_size {
                if db.len() > max_size {
                    db.remove(0);
                }
            }
        })
        .map_err(ErrorSequentialState::WriteFailures)?;
        db.save().map_err(ErrorSequentialState::SaveFailures)
    }

    #[allow(dead_code)]
    pub fn last(&self) -> ModResult<Option<V>> {
        let db = self.open_db()?;
        db.read(|db| db.last().map(Clone::clone))
            .map_err(ErrorSequentialState::ReadFailure)
    }

    #[allow(dead_code)]
    pub fn first(&self) -> ModResult<Option<V>> {
        let db = self.open_db()?;
        db.read(|db| db.first().map(Clone::clone))
            .map_err(ErrorSequentialState::ReadFailure)
    }

    pub fn entries(&self) -> ModResult<impl Iterator<Item = V>> {
        let db = self.open_db()?;
        db.read(|db| db.clone().into_iter())
            .map_err(ErrorSequentialState::ReadFailure)
    }

    #[allow(dead_code)]
    pub fn delete(&self, position: usize) -> ModResult<()> {
        let db = self.open_db()?;
        db.write(|db| {
            db.remove(position);
        })
        .map_err(ErrorSequentialState::WriteFailures)?;
        db.save().map_err(ErrorSequentialState::SaveFailures)
    }

    fn open_db(&self) -> ModResult<Fdb<V>> {
        Fdb::<V>::load_from_path(&self.path)
            .or_else(|_| Fdb::<V>::create_at_path(&self.path, vec![]))
            .map_err(ErrorSequentialState::OpenFailure)
    }
}

#[cfg(test)]
mod tests {
    use sam_utils::fsutils::TempFile;

    use super::{ModResult, SequentialState, Value};

    fn make_temp_state<V: Value>() -> SequentialState<V> {
        let f = TempFile::new().expect("failed to created a temporary file");
        SequentialState::new(f.path, None).expect("failed to create a new db")
    }

    fn insert_values<V: Value>(state: &SequentialState<V>, values: &[V]) -> ModResult<()> {
        for v in values {
            state.push(v.clone())?;
        }
        Ok(())
    }

    #[test]
    fn test_sequential_state() {
        let values = vec![1, 2, 3, 4, 7];
        let state = make_temp_state::<i32>();
        insert_values(&state, &values).expect("could not into state");
        let returned_values: Vec<i32> =
            state.entries().expect("call to into_iter failed").collect();
        assert_eq!(returned_values, values);

        state.delete(1).expect("could not delete from state");

        let values = vec![1, 3, 4, 7];
        let returned_values: Vec<i32> =
            state.entries().expect("call to into_iter failed").collect();
        assert_eq!(returned_values, values);
    }

    #[test]
    fn test_sequential_state_first_last() {
        let values = vec![1, 2, 3, 4, 7];
        let state = make_temp_state::<i32>();
        insert_values(&state, &values).expect("could not into state");

        assert_eq!(state.first().expect("could not get first element"), Some(1));
        assert_eq!(state.last().expect("could not get last element"), Some(7));
    }
}
