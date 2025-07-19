use sam_core::entities::choices::Choice;
use sam_core::entities::identifiers::Identifier;
use sam_persistence::{SessionError, SessionStorage};
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub enum SessionCommand {
    Set { var_name: String, choice_value: String },
    Clear,
    List,
}

pub struct SessionEngine {
    storage: SessionStorage,
}

impl SessionEngine {
    pub fn new(session_dir: impl AsRef<Path>, ttl: Duration) -> Result<Self, ErrorSessionEngine> {
        let storage = SessionStorage::with_ttl(session_dir, &ttl)?;
        Ok(SessionEngine { storage })
    }

    pub fn run(&self, command: SessionCommand) -> Result<i32, ErrorSessionEngine> {
        match command {
            SessionCommand::Set { var_name, choice_value } => {
                let identifier = Identifier::new(var_name.clone());
                let choice = Choice::from_value(choice_value.clone());
                self.storage.set_choice(identifier, choice)?;
                println!("Session default set: {} = {}", var_name, choice_value);
                Ok(0)
            }
            SessionCommand::Clear => {
                self.storage.clear_session()?;
                println!("Session defaults cleared for session: {}", self.storage.session_id());
                Ok(0)
            }
            SessionCommand::List => {
                let choices = self.storage.get_all_choices()?;
                if choices.is_empty() {
                    println!("No session defaults set for session: {}", self.storage.session_id());
                } else {
                    println!("Session defaults for session: {}", self.storage.session_id());
                    for (var_name, choice) in choices {
                        println!("  {} = {}", var_name, choice);
                    }
                }
                Ok(0)
            }
        }
    }

    pub fn get_session_defaults(&self) -> Result<std::collections::HashMap<Identifier, Vec<Choice>>, ErrorSessionEngine> {
        let session_choices = self.storage.get_all_choices()?;
        // Convert single choices to Vec<Choice> to match expected format
        let mut defaults = std::collections::HashMap::new();
        for (identifier, choice) in session_choices {
            defaults.insert(identifier, vec![choice]);
        }
        Ok(defaults)
    }
}

#[derive(Debug, Error)]
pub enum ErrorSessionEngine {
    #[error("Session storage error: {0}")]
    Storage(#[from] SessionError),
}