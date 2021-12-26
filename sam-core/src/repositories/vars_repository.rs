use crate::choices::Choice;
use crate::commands::Command;
use crate::dependencies::{Dependencies, ErrorsResolver, Resolver};
use crate::engines::{ErrorsVarsRepositoryT, VarsRepositoryT};
use crate::identifiers::{Identifier, Identifiers};
use crate::processes::ShellCommand;
use crate::vars::Var;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

#[derive(Debug, Default)]
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

    /// will return a valid choice for the current Var using the provided VarResolver and the
    /// HashMap of choices provided.
    /// First, this function will look into the `choices` HashMap to fill values for all the dependencies of the current
    /// `Var`and then use the resolver to get a `Choice` for the current `Var`
    pub fn resolve<'repository, R>(
        resolver: &'repository R,
        var: &'repository Var,
        choices: &'repository HashMap<Identifier, Choice>,
    ) -> Result<Choice, ErrorsVarsRepositoryT>
    where
        R: Resolver,
    {
        Self::_resolve(resolver, var, choices).map_err(|err| {
            ErrorsVarsRepositoryT::NoChoiceForVar {
                var_name: var.name(),
                error: err,
            }
        })
    }

    fn _resolve<'repository, R>(
        resolver: &'repository R,
        var: &'repository Var,
        choices: &'repository HashMap<Identifier, Choice>,
    ) -> Result<Choice, ErrorsResolver>
    where
        R: Resolver,
    {
        if var.is_command() {
            let command = var.substitute_for_choices(choices)?;
            resolver.resolve_dynamic(var.name(), ShellCommand::new(command))
        } else if var.is_input() {
            let prompt = var.prompt().unwrap_or("no provided prompt");
            resolver.resolve_input(var.name(), prompt)
        } else {
            resolver.resolve_static(var.name(), var.choices().into_iter())
        }
    }

    pub fn vars_iter(&self) -> impl Iterator<Item = &Var> {
        self.vars.iter()
    }
}

impl VarsRepositoryT for VarsRepository {
    fn execution_sequence<Deps: Dependencies>(
        &self,
        dep: Deps,
    ) -> std::result::Result<
        crate::dependencies::ExecutionSequence<'_>,
        crate::engines::ErrorsVarsRepositoryT,
    > {
        let mut already_seen = HashSet::new();
        let mut already_inserted = HashSet::new();
        let mut candidates = dep.dependencies();
        let mut missing = Vec::default();
        let mut execution_seq = VecDeque::default();

        while let Some(cur) = candidates.pop() {
            if already_seen.contains(&cur) && !already_inserted.contains(&cur) {
                already_inserted.insert(cur.clone());
                if let Some(cur_var) = self.vars.get(&cur) {
                    execution_seq.push_back(Borrow::borrow(cur_var));
                }
                continue;
            }
            if already_seen.contains(&cur) {
                continue;
            }
            if let Some(cur_var) = self.vars.get(&cur) {
                let deps = cur_var.dependencies();
                already_seen.insert(cur.clone());
                if deps.is_empty() {
                    already_inserted.insert(cur.clone());
                    execution_seq.push_front(Borrow::borrow(cur_var));
                } else {
                    candidates.push(cur);
                    candidates.extend_from_slice(deps.as_slice());
                }
            } else {
                missing.push(cur);
            }
        }

        if !missing.is_empty() {
            Err(ErrorsVarsRepositoryT::MissingDependencies(Identifiers(
                missing,
            )))
        } else {
            Ok(crate::dependencies::ExecutionSequence::new(
                execution_seq.into_iter().collect(),
            ))
        }
    }

    // choices uses the provided resolver to fetch choices for
    // the provided `ExecutionSequence`.
    fn choices<'repository, R>(
        &'repository self,
        resolver: &'repository R,
        vars: crate::dependencies::ExecutionSequence<'repository>,
    ) -> Result<Vec<(Identifier, Choice)>, ErrorsVarsRepositoryT>
    where
        R: Resolver,
    {
        let mut choices: HashMap<Identifier, Choice> = HashMap::new();
        for var_name in vars.as_slice() {
            if let Some(var) = self.vars.get(*var_name) {
                let choice = if let Some(default) = self.defaults.get(&var.name()) {
                    default.to_owned()
                } else {
                    Self::resolve(resolver, var, &choices)?
                };
                choices.insert(var.name(), choice);
            } else {
                return Err(ErrorsVarsRepositoryT::MissingDependencies(Identifiers(
                    vec![var_name.clone().to_owned()],
                )));
            }
        }
        Ok(choices.into_iter().collect())
    }
    fn set_defaults(
        &mut self,
        defaults: &HashMap<Identifier, Choice>,
    ) -> std::result::Result<(), crate::engines::ErrorsVarsRepositoryT> {
        let mut identifiers = vec![];
        for key in defaults.keys() {
            if !self.vars.contains(key) {
                identifiers.push(key.clone());
            }
        }
        if identifiers.is_empty() {
            self.defaults = defaults.to_owned();
            Ok(())
        } else {
            Err(ErrorsVarsRepositoryT::UnknowVarsDefaults(Identifiers {
                0: identifiers,
            }))
        }
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
    use crate::dependencies::mocks::StaticResolver;
    use crate::identifiers::fixtures::*;
    use crate::vars::fixtures::*;
    use maplit::hashmap;

    #[test]
    fn test_resolve() {
        let choices = hashmap! {
            VAR_DIRECTORY_NAME.clone() => VAR_DIRECTORY_CHOICE_1.clone(),
            VAR_PATTERN_NAME.clone() => VAR_PATTERN_CHOICE_2.clone(),
        };
        let command_final = format!(
            "ls -l {} |grep -v {}",
            VAR_DIRECTORY_CHOICE_1.value(),
            VAR_PATTERN_CHOICE_2.value()
        );
        let choice_final = Choice::from_value("final_value");
        let dynamic_res = hashmap![
            command_final => choice_final.clone(),
        ];
        let static_res = hashmap![
            VAR_DIRECTORY_NAME.clone() => VAR_DIRECTORY_CHOICE_1.clone(),
            VAR_PATTERN_NAME.clone() => VAR_PATTERN_CHOICE_2.clone(),
        ];
        let resolver = StaticResolver::new(dynamic_res, static_res, None);
        let var1 = VAR_LISTING.clone();
        let ret_var1 = VarsRepository::resolve(&resolver, &var1, &choices);
        assert!(ret_var1.is_ok());
        assert_eq!(ret_var1.unwrap(), choice_final);
        let var2 = VAR_PATTERN.clone();
        let ret_var2 = VarsRepository::resolve(&resolver, &var2, &choices);
        assert!(ret_var2.is_ok());
        assert_eq!(ret_var2.unwrap(), VAR_PATTERN_CHOICE_2.clone());
    }
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
            _ => assert!(false),
        }
    }

    #[test]
    fn test_var_repository_execution_sequence() {
        let full = vec![
            VAR_DIRECTORY.clone(),
            VAR_LISTING.clone(),
            VAR_PATTERN.clone(),
        ];
        let repo = VarsRepository::new(full.into_iter());
        let seq = repo.execution_sequence(VAR_LISTING.clone());
        assert!(seq.is_ok());
        let seq = repo.execution_sequence(VAR_USE_LISTING.clone());
        assert!(seq.is_ok());
        let expected = vec![
            VAR_DIRECTORY_NAME.clone(),
            VAR_PATTERN_NAME.clone(),
            VAR_LISTING_NAME.clone(),
        ];
        assert_eq!(expected.iter().as_slice(), seq.unwrap().as_ref());
    }
    #[test]
    fn test_var_repository_choices() {
        let choice_final = Choice::from_value("final_value");
        let command_final = format!(
            "ls -l {} |grep -v {}",
            VAR_DIRECTORY_CHOICE_1.value(),
            VAR_PATTERN_CHOICE_2.value()
        );
        let dynamic_res = hashmap![
            command_final => choice_final.clone(),
        ];
        let static_res = hashmap![
            VAR_DIRECTORY_NAME.clone() => VAR_DIRECTORY_CHOICE_1.clone(),
            VAR_PATTERN_NAME.clone() => VAR_PATTERN_CHOICE_2.clone(),
        ];
        let resolver = StaticResolver::new(dynamic_res, static_res, None);
        let full = vec![
            VAR_DIRECTORY.clone(),
            VAR_LISTING.clone(),
            VAR_PATTERN.clone(),
        ];
        let repo = VarsRepository::new(full.into_iter());
        let seq = repo.execution_sequence(VAR_USE_LISTING.clone()).unwrap();
        let res = repo.choices(&resolver, seq);
        assert!(res.is_ok());
        let expected = vec![
            (VAR_PATTERN_NAME.clone(), VAR_PATTERN_CHOICE_2.clone()),
            (VAR_LISTING_NAME.clone(), choice_final),
            (VAR_DIRECTORY_NAME.clone(), VAR_DIRECTORY_CHOICE_1.clone()),
        ]
        .sort();
        assert_eq!(res.unwrap().sort(), expected);
    }
}

pub mod fixtures {}
