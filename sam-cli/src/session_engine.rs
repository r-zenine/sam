use sam_core::algorithms::VarsCollection;
use sam_core::engines::SessionSaver;
use sam_core::entities::choices::Choice;
use sam_core::entities::identifiers::Identifier;
use sam_persistence::repositories::VarsRepository;
use sam_persistence::{SessionError, SessionStorage};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub enum SessionCommand {
    Set {
        var_name: String,
        choice_value: String,
    },
    Clear,
    List,
}

pub struct SessionEngine {
    storage: SessionStorage,
    vars_repository: VarsRepository,
}

impl SessionEngine {
    pub fn new(session_dir: impl AsRef<Path>, ttl: Duration, vars_repository: VarsRepository) -> Result<Self, ErrorSessionEngine> {
        let storage = SessionStorage::with_ttl(session_dir, &ttl)?;
        Ok(SessionEngine { storage, vars_repository })
    }

    pub fn run(&self, command: SessionCommand) -> Result<i32, ErrorSessionEngine> {
        match command {
            SessionCommand::Set {
                var_name,
                choice_value,
            } => {
                let identifier = Identifier::from_str(&var_name);
                
                // Check if the variable exists in the configuration
                if self.vars_repository.get(&identifier).is_none() {
                    return Err(ErrorSessionEngine::VariableNotFound(var_name));
                }
                
                let choice = Choice::from_value(choice_value.clone());
                self.storage.set_choice(identifier, choice)?;
                println!("Session default set: {} = {}", var_name, choice_value);
                Ok(0)
            }
            SessionCommand::Clear => {
                self.storage.clear_session()?;
                println!(
                    "Session defaults cleared for session: {}",
                    self.storage.session_id()
                );
                Ok(0)
            }
            SessionCommand::List => {
                let choices = self.storage.get_all_choices()?;
                if choices.is_empty() {
                    println!(
                        "No session defaults set for session: {}",
                        self.storage.session_id()
                    );
                } else {
                    println!(
                        "Session defaults for session: {}",
                        self.storage.session_id()
                    );
                    for (var_name, choice) in choices {
                        println!("  {} = {}", var_name, choice);
                    }
                }
                Ok(0)
            }
        }
    }

    pub fn get_session_defaults(
        &self,
    ) -> Result<std::collections::HashMap<Identifier, Vec<Choice>>, ErrorSessionEngine> {
        let session_choices = self.storage.get_all_choices()?;
        // Convert single choices to Vec<Choice> to match expected format
        let mut defaults = std::collections::HashMap::new();
        for (identifier, choice) in session_choices {
            defaults.insert(identifier, vec![choice]);
        }
        Ok(defaults)
    }
}

impl SessionSaver for SessionEngine {
    fn save_choices(
        &self,
        choices: &HashMap<Identifier, Vec<Choice>>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        for (identifier, choice_vec) in choices {
            // Only save the first choice (user's selection)
            if let Some(choice) = choice_vec.first() {
                self.storage
                    .set_choice(identifier.clone(), choice.clone())?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ErrorSessionEngine {
    #[error("Session storage error: {0}")]
    Storage(#[from] SessionError),
    #[error("you are trying to set a Variable '{0}' that does not exist in the configuration.")]
    VariableNotFound(String),
}
