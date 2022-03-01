use sam_core::algorithms::{VarsCollection, VarsDefaultValues};
use sam_core::engines::VarsDefaultValuesSetter;
use sam_core::entities::choices::Choice;
use sam_core::entities::commands::Command;
use sam_core::entities::dependencies::ErrorsResolver;
use sam_core::entities::identifiers::{Identifier, Identifiers};
use sam_core::entities::vars::Var;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Default, Clone)]
pub struct VarsRepository {
    vars: HashSet<Var>,
    defaults: HashMap<Identifier, Choice>,
}

impl VarsRepository {
    /// new creates a var Repository. this function will return an `ErrorVarRepository::ErrorMissingDependencies`
    /// if a Var provided has a dependency that is not found in the Iterator.
    pub fn new(value: impl Iterator<Item = Var>) -> Self {
        let vars: HashSet<Var> = value.collect();
        VarsRepository {
            vars,
            defaults: HashMap::default(),
        }
    }

    pub fn with_defaults(
        value: impl Iterator<Item = Var>,
        defaults: HashMap<Identifier, Choice>,
    ) -> Self {
        let vars: HashSet<Var> = value.collect();
        VarsRepository { vars, defaults }
    }

    pub fn merge(&mut self, other: VarsRepository) {
        self.vars.extend(other.vars);
    }

    pub fn ensure_no_missing_dependency(&self) -> Result<(), ErrorsVarsRepository> {
        let missing: Vec<Identifier> = self
            .vars
            .iter()
            .flat_map(Var::dependencies)
            .filter(|e| !self.vars.contains(e))
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(ErrorsVarsRepository::MissingDependencies(Identifiers(
                missing,
            )))
        }
    }

    pub fn vars_iter(&self) -> impl Iterator<Item = &Var> {
        self.vars.iter()
    }
}

impl VarsDefaultValuesSetter for VarsRepository {
    fn set_defaults(&mut self, defaults: &HashMap<Identifier, Choice>) {
        let mut identifiers = vec![];
        for key in defaults.keys() {
            if !self.vars.contains(key) {
                identifiers.push(key.clone());
            }
        }
        self.defaults = defaults.to_owned();
    }
}

impl VarsDefaultValues for VarsRepository {
    fn default_value(&self, id: &Identifier) -> Option<&Choice> {
        self.defaults.get(id)
    }
}

impl VarsCollection for VarsRepository {
    fn get(&self, id: &Identifier) -> Option<&Var> {
        self.vars.get(id)
    }
}

#[derive(Debug, Error)]
pub enum ErrorsVarsRepository {
    #[error("missing the following dependencies:\n{0}")]
    MissingDependencies(Identifiers),
    #[error("the provided variables are unknown:\n{0}")]
    UnknowVarsDefaults(Identifiers),
    #[error("no choices available for var {var_name}\n-> {error}")]
    NoChoiceForVar {
        var_name: Identifier,
        error: ErrorsResolver,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use sam_core::entities::identifiers::fixtures::*;
    use sam_core::entities::vars::fixtures::*;

    #[test]
    fn test_var_repository_new() {
        let full = vec![
            VAR_DIRECTORY.clone(),
            VAR_LISTING.clone(),
            VAR_PATTERN.clone(),
        ];
        let _repo = VarsRepository::new(full.into_iter());
        let missing = vec![VAR_DIRECTORY.clone(), VAR_LISTING.clone()];
        let repo_err = VarsRepository::new(missing.into_iter());
        let missing_err = repo_err.ensure_no_missing_dependency();
        assert!(missing_err.is_err());
        match missing_err.unwrap_err() {
            ErrorsVarsRepository::MissingDependencies(identifiers) => {
                assert_eq!(identifiers, Identifiers(vec![VAR_PATTERN_NAME.clone()]));
            }
            _ => unreachable!(),
        }
    }
}
