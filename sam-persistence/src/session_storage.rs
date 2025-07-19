use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

use crate::associative_state::AssociativeStateWithTTL;
use crate::associative_state::ErrorAssociativeState;
use sam_core::entities::choices::Choice;
use sam_core::entities::identifiers::Identifier;

/// SessionStorage provides persistent storage for variable choices within a terminal session
#[derive(Debug)]
pub struct SessionStorage {
    state: AssociativeStateWithTTL<SessionEntry>,
    session_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SessionEntry {
    pub var_name: Identifier,
    pub choice: Choice,
    pub session_id: String,
}

impl SessionStorage {
    /// Create a new SessionStorage with the given path and TTL
    /// Session TTL is typically longer than cache TTL (e.g., 24 hours)
    pub fn with_ttl(p: impl AsRef<Path>, ttl: &Duration) -> Result<Self, SessionError> {
        let session_id = Self::get_session_id();
        Ok(SessionStorage {
            state: AssociativeStateWithTTL::<SessionEntry>::with_ttl(p, ttl)?,
            session_id,
        })
    }

    /// Store a variable choice for the current session
    pub fn set_choice(&self, var_name: Identifier, choice: Choice) -> Result<(), SessionError> {
        let key = self.make_key(&var_name);
        let entry = SessionEntry {
            var_name: var_name.clone(),
            choice,
            session_id: self.session_id.clone(),
        };
        self.state.put(key, entry)?;
        Ok(())
    }

    /// Get a variable choice for the current session
    pub fn get_choice(&self, var_name: &Identifier) -> Result<Option<Choice>, SessionError> {
        let key = self.make_key(var_name);
        if let Some(entry) = self.state.get(&key)? {
            // Verify the entry belongs to the current session
            if entry.session_id == self.session_id {
                Ok(Some(entry.choice))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Get all variable choices for the current session
    pub fn get_all_choices(&self) -> Result<HashMap<Identifier, Choice>, SessionError> {
        let mut result = HashMap::new();
        for (_, entry) in self.state.entries()? {
            if entry.session_id == self.session_id {
                result.insert(entry.var_name.clone(), entry.choice);
            }
        }
        Ok(result)
    }

    /// Clear all choices for the current session
    pub fn clear_session(&self) -> Result<(), SessionError> {
        let keys_to_delete: Vec<String> = self
            .state
            .entries()?
            .filter_map(|(key, entry)| {
                if entry.session_id == self.session_id {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();

        for key in keys_to_delete {
            self.state.delete(&key)?;
        }
        Ok(())
    }

    /// Clear all sessions (admin function)
    pub fn clear_all(&self) -> Result<(), SessionError> {
        for (key, _) in self.state.entries()? {
            self.state.delete(key)?;
        }
        Ok(())
    }

    /// Get current session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Generate a key for storing session data
    fn make_key(&self, var_name: &Identifier) -> String {
        format!("{}:{}", self.session_id, var_name)
    }

    /// Get the current terminal session identifier
    /// This uses environment variables that are more stable across processes in the same session
    fn get_session_id() -> String {
        // Try to get a stable session identifier
        // Use terminal session ID if available (more stable than PPID)
        if let Ok(term_session) = std::env::var("TERM_SESSION_ID") {
            // macOS Terminal provides this
            term_session
        } else if let Ok(tmux_pane) = std::env::var("TMUX_PANE") {
            // tmux session
            tmux_pane
        } else if let Ok(ssh_client) = std::env::var("SSH_CLIENT") {
            // SSH session - use client info as session id
            format!("ssh_{}", ssh_client.replace(' ', "_"))
        } else if let Ok(ppid) = std::env::var("PPID") {
            // Fallback to PPID (parent process ID)
            ppid
        } else {
            // Last resort: use a fixed session for this terminal
            // In real usage, this would be the shell's process ID
            "terminal_session".to_string()
        }
    }
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session storage error: {0}")]
    Storage(#[from] ErrorAssociativeState),
}

#[cfg(test)]
mod tests {
    use super::*;
    use sam_core::entities::identifiers::Identifier;
    use tempfile::tempdir;

    #[test]
    fn test_session_storage() {
        let temp_dir = tempdir().unwrap();
        let session_path = temp_dir.path().join("session_test");
        let ttl = Duration::from_secs(3600);
        
        let storage = SessionStorage::with_ttl(&session_path, &ttl).unwrap();
        
        let var_name = Identifier::new("test_var");
        let choice = Choice::new("test_value", Some("test description"));
        
        // Test setting and getting a choice
        storage.set_choice(var_name.clone(), choice.clone()).unwrap();
        let retrieved = storage.get_choice(&var_name).unwrap();
        assert_eq!(retrieved, Some(choice.clone()));
        
        // Test getting all choices
        let all_choices = storage.get_all_choices().unwrap();
        assert_eq!(all_choices.len(), 1);
        assert_eq!(all_choices.get(&var_name), Some(&choice));
        
        // Test clearing session
        storage.clear_session().unwrap();
        let cleared = storage.get_choice(&var_name).unwrap();
        assert_eq!(cleared, None);
    }

    #[test]
    fn test_session_isolation() {
        let temp_dir = tempdir().unwrap();
        let session_path = temp_dir.path().join("session_isolation_test");
        let ttl = Duration::from_secs(3600);
        
        // Create storage instances that would have different session IDs in real usage
        let storage1 = SessionStorage::with_ttl(&session_path, &ttl).unwrap();
        let storage2 = SessionStorage::with_ttl(&session_path, &ttl).unwrap();
        
        // They should have the same session ID in tests (same process)
        assert_eq!(storage1.session_id(), storage2.session_id());
        
        let var_name = Identifier::new("test_var");
        let choice = Choice::new("test_value", Some("test description"));
        
        storage1.set_choice(var_name.clone(), choice.clone()).unwrap();
        
        // Both should see the same data since they're in the same session
        let retrieved1 = storage1.get_choice(&var_name).unwrap();
        let retrieved2 = storage2.get_choice(&var_name).unwrap();
        
        assert_eq!(retrieved1, Some(choice.clone()));
        assert_eq!(retrieved2, Some(choice));
    }
}