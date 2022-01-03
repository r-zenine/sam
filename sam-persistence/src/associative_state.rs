use rustbreak::RustbreakError;
use rustbreak::{deser::Ron, FileDatabase};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use thiserror::Error;

#[derive(Debug)]
pub struct AssociativeStateWithTTL<V> {
    path: PathBuf,
    ttl: Option<Duration>,
    _marker: PhantomData<V>,
}

#[derive(Error, Debug)]
pub enum ErrorAssociativeState {
    #[error("failed to create associative state because\n->{0}")]
    CreationFailure(RustbreakError),
    #[error("failed to initialize associative state because\n->{0}")]
    InitFailure(RustbreakError),
    #[error("failed to load associative state because\n->{0}")]
    OpenFailure(RustbreakError),
    #[error("failed to write to associative state because\n->{0}")]
    WriteFailures(RustbreakError),
    #[error("failed to save to associative state because\n->{0}")]
    SaveFailures(RustbreakError),
    #[error("failed to read from associative state because\n->{0}")]
    ReadFailure(RustbreakError),
}

pub trait Value: Serialize + DeserializeOwned + Send + Clone + std::fmt::Debug {}
impl<T> Value for T where T: Serialize + DeserializeOwned + Send + Clone + std::fmt::Debug {}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct StateEntry<V> {
    entry: V,
    when: u64,
}

impl<V> StateEntry<V> {
    pub fn new(value: V) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("can't get system time");
        StateEntry {
            entry: value,
            when: now.as_secs(),
        }
    }
}

type FDB<V> = FileDatabase<HashMap<String, StateEntry<V>>, Ron>;

impl<V> AssociativeStateWithTTL<V>
where
    V: Value,
{
    pub fn with_ttl(p: impl AsRef<Path>, ttl: &Duration) -> Result<Self, ErrorAssociativeState> {
        let db = AssociativeStateWithTTL {
            path: p.as_ref().to_owned(),
            ttl: Some(*ttl),
            _marker: PhantomData::default(),
        };
        db.open_db()?;
        Ok(db)
    }

    pub fn new(p: impl AsRef<Path>) -> Result<Self, ErrorAssociativeState> {
        let db = AssociativeStateWithTTL {
            path: p.as_ref().to_owned(),
            ttl: None,
            _marker: PhantomData::default(),
        };
        db.open_db()?;
        Ok(db)
    }

    pub fn put(&self, key: impl AsRef<str>, value: V) -> Result<(), ErrorAssociativeState> {
        let db = self.open_db()?;
        let entry = StateEntry::new(value);
        db.write(|db| {
            db.insert(key.as_ref().to_string(), entry);

            let mut keys_to_drop = vec![];
            for (key, value) in db.iter() {
                if !self.is_value_valid(value) {
                    keys_to_drop.push(key.clone());
                }
            }

            for key in keys_to_drop {
                db.remove(&key);
            }
        })
        .map_err(ErrorAssociativeState::WriteFailures)?;
        db.save().map_err(ErrorAssociativeState::SaveFailures)
    }

    pub fn get(&self, command: impl AsRef<str>) -> Result<Option<V>, ErrorAssociativeState> {
        let db = self.open_db()?;
        let cache_key = command.as_ref();
        let entry = db
            .read(|db| db.get(cache_key).map(Clone::clone))
            .map_err(ErrorAssociativeState::ReadFailure)?;
        Ok(entry.filter(|v| self.is_value_valid(v)).map(|e| e.entry))
    }

    pub fn delete(&self, key: impl AsRef<str>) -> Result<Option<V>, ErrorAssociativeState> {
        let db = self.open_db()?;
        let cache_key = key.as_ref();
        let entry = db
            .write(|db| db.remove(cache_key))
            .map_err(ErrorAssociativeState::WriteFailures)?;
        db.save().map_err(ErrorAssociativeState::SaveFailures)?;
        Ok(entry.filter(|v| self.is_value_valid(v)).map(|e| e.entry))
    }

    pub fn entries(&self) -> Result<impl Iterator<Item = (String, V)>, ErrorAssociativeState> {
        let db = self.open_db()?;
        db.read(|db| db.clone().into_iter().map(|(k, v)| (k, v.entry)))
            .map_err(ErrorAssociativeState::ReadFailure)
    }

    fn open_db(&self) -> Result<FDB<V>, ErrorAssociativeState> {
        Ok(FDB::<V>::load_from_path(&self.path)
            .or(FDB::<V>::create_at_path(&self.path, HashMap::default()))
            .map_err(ErrorAssociativeState::OpenFailure)?)
    }
    fn is_value_valid(&self, c: &StateEntry<V>) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Can't get system time");
        if let Some(ttl) = self.ttl.as_ref() {
            c.when + ttl.as_secs() > now.as_secs()
        } else {
            true
        }
    }
}

pub trait EntrySelector<V> {
    fn select_entry(
        &self,
        data: impl Iterator<Item = (String, V)>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>>;
}

pub trait EntryUpdater<V> {
    fn update_value(&self, key: &str, value: &V) -> Result<Option<V>, Box<dyn std::error::Error>>;
}

pub struct AssociativeStateInteractor<V, D> {
    state: AssociativeStateWithTTL<V>,
    delegate: D,
}

impl<V, D> AssociativeStateInteractor<V, D> {
    fn new(path: impl AsRef<Path>, ttl: Option<Duration>, delegate: D) -> Self {
        AssociativeStateInteractor {
            state: AssociativeStateWithTTL {
                path: path.as_ref().to_path_buf(),
                ttl,
                _marker: PhantomData::default(),
            },
            delegate,
        }
    }
}

impl<V, D> AssociativeStateInteractor<V, D>
where
    D: EntrySelector<V>,
    V: Value,
{
    fn delete_entry(&self) -> Result<(), Box<dyn std::error::Error>> {
        let state_iterator = self.entries()?;
        let selected_element = self.delegate.select_entry(state_iterator)?;
        if let Some(key) = selected_element {
            self.state.delete(key)?;
        }
        Ok(())
    }
}

impl<V, D> AssociativeStateInteractor<V, D>
where
    D: EntrySelector<V> + EntryUpdater<V>,
    V: Value,
{
    fn update_entry(&self) -> Result<(), Box<dyn std::error::Error>> {
        let state_iterator = self.entries()?;
        let selected_key = self.delegate.select_entry(state_iterator)?;
        let selected_value = selected_key
            .as_deref()
            .and_then(|k| self.state.get(k).transpose())
            .transpose()?;
        if let Some((k, v)) = selected_key.zip(selected_value) {
            let updated_value = self.delegate.update_value(&k, &v)?;
            updated_value.map(|v| self.state.put(k, v));
        }
        Ok(())
    }
}

impl<V, D> AssociativeStateInteractor<V, D>
where
    V: Value,
{
    fn entries(&self) -> Result<impl Iterator<Item = (String, V)>, ErrorAssociativeState> {
        self.state.entries()
    }
}

#[cfg(test)]
mod tests {
    use sam_utils::fsutils::TempFile;

    use super::{AssociativeStateWithTTL, Value};

    fn make_temp_state<V: Value>() -> AssociativeStateWithTTL<V> {
        let f = TempFile::new().expect("failed to created a temporary file");
        AssociativeStateWithTTL::new(f.path).expect("failed to create a new db")
    }

    #[test]
    fn test_associative_state() {
        let db = make_temp_state::<i32>();
        let values = vec![
            (String::from("str"), 1),
            (String::from("str_1"), 2),
            (String::from("str_2"), 3),
        ];

        for (k, v) in &values {
            db.put(k, *v).expect("could not put");
        }

        let entries: Vec<(String, i32)> = db.entries().expect("can't get entries").collect();
        assert_eq!(entries.clone().sort(), values.clone().sort());

        let value = db
            .get(String::from("str"))
            .expect("can't get data from state")
            .expect("Got a None when I expected a value");

        assert_eq!(value, 1);

        let value2 = db
            .delete(String::from("str"))
            .expect("can't get data from state")
            .expect("Got a None when I expected a value");

        assert_eq!(value2, 1);
        assert!(db
            .get(String::from("str"))
            .expect("can't get data from state")
            .is_none());
    }

    #[test]
    fn test_associative_state_interactor_delete_entry() {}

    #[test]
    fn test_associative_state_interactor_update_entry() {}

    #[test]
    fn test_associative_state_interactor_entries() {}
}
