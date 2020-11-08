use crate::core::choices::Choice;
use crate::core::commands::Command;
use crate::core::dependencies::{Dependencies, ErrorsResolver, Resolver};
use crate::core::identifiers::{Identifier, Identifiers};
use crate::core::vars::Var;
use crate::utils::processes::ShellCommand;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;
#[derive(Debug)]
pub struct ExecutionSequence<'repository> {
    inner: Vec<&'repository Identifier>,
}
#[derive(Debug, Default)]
pub struct VarsRepository {
    vars: HashSet<Var>,
}

impl<'repository> AsRef<[&'repository Identifier]> for ExecutionSequence<'repository> {
    fn as_ref(&self) -> &[&'repository Identifier] {
        self.inner.as_slice()
    }
}

// TODO tests for this.
impl VarsRepository {
    /// new creates a var Repository. this function will return an `ErrorVarRepository::ErrorMissingDependencies`
    /// if a Var provided has a dependency that is not found in the Iterator.
    pub fn new(value: impl Iterator<Item = Var>) -> Result<Self, ErrorsVarsRepository> {
        let vars: HashSet<Var> = value.collect();
        let missing: Vec<Identifier> = vars
            .iter()
            .flat_map(Var::dependencies)
            .filter(|e| !vars.contains(e))
            .collect();
        if missing.is_empty() {
            Ok(VarsRepository { vars })
        } else {
            Err(ErrorsVarsRepository::MissingDependencies(Identifiers(
                missing,
            )))
        }
    }

    pub fn merge(&mut self, other: VarsRepository) {
        self.vars.extend(other.vars);
    }

    /// Execution sequence returns for a given `Dep: Dependencies`
    /// an execution sequence of VARs in order to fulfill it's dependencies.
    pub fn execution_sequence<'repository, Deps>(
        &'repository self,
        dep: Deps,
    ) -> Result<ExecutionSequence<'repository>, ErrorsVarsRepository>
    where
        Deps: Dependencies,
    {
        let mut already_seen = HashSet::new();
        let mut candidates = dep.dependencies();
        let mut missing = Vec::default();
        let mut execution_seq = VecDeque::default();
        let mut push_front = 0;

        while let Some(cur) = candidates.pop() {
            if already_seen.contains(&cur) {
                continue;
            }
            if let Some(cur_var) = self.vars.get(&cur) {
                let deps = cur_var.dependencies();
                already_seen.insert(cur);
                if deps.is_empty() {
                    execution_seq.push_front(Borrow::borrow(cur_var));
                    push_front += 1;
                } else {
                    candidates.extend_from_slice(deps.as_slice());
                    execution_seq.insert(push_front, Borrow::borrow(cur_var));
                }
            } else {
                missing.push(cur);
            }
        }

        if !missing.is_empty() {
            Err(ErrorsVarsRepository::MissingDependencies(Identifiers(
                missing,
            )))
        } else {
            Ok(ExecutionSequence {
                inner: execution_seq.into_iter().collect(),
            })
        }
    }

    // choices uses the provided resolver to fetch choices for
    // the provided `ExecutionSequence`.
    pub fn choices<'repository, R>(
        &'repository self,
        resolver: &'repository R,
        vars: ExecutionSequence<'repository>,
    ) -> Result<Vec<(Identifier, Choice)>, ErrorsVarsRepository>
    where
        R: Resolver,
    {
        let mut choices: HashMap<Identifier, Choice> = HashMap::new();
        for var_name in vars.inner {
            if let Some(var) = self.vars.get(var_name) {
                let choice = Self::resolve(resolver, var, &choices)?;
                choices.insert(var_name.to_owned(), choice);
            } else {
                return Err(ErrorsVarsRepository::MissingDependencies(Identifiers(
                    vec![var_name.to_owned()],
                )));
            }
        }
        Ok(choices.into_iter().collect())
    }
    /// will return a valid choice for the current Var using the provided VarResolver and the
    /// HashMap of choices provided.
    /// First, this function will look into the `choices` HashMap to fill values for all the dependencies of the current
    /// `Var`and then use the resolver to get a `Choice` for the current `Var`
    pub fn resolve<'repository, R>(
        resolver: &'repository R,
        var: &'repository Var,
        choices: &'repository HashMap<Identifier, Choice>,
    ) -> Result<Choice, ErrorsVarsRepository>
    where
        R: Resolver,
    {
        Self::_resolve(resolver, var, choices).map_err(|err| ErrorsVarsRepository::NoChoiceForVar {
            var_name: var.name(),
            error: err,
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
        } else {
            resolver.resolve_static(var.name(), var.choices().into_iter())
        }
    }
}

#[derive(Debug, PartialEq, Error)]
pub enum ErrorsVarsRepository {
    #[error("missing the following dependencies:\n{0}")]
    MissingDependencies(Identifiers),
    #[error("no choices available for var {var_name}\n->{error}")]
    NoChoiceForVar {
        var_name: Identifier,
        error: ErrorsResolver,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dependencies::mocks::StaticResolver;
    use crate::core::identifiers::fixtures::*;
    use crate::core::vars::fixtures::*;
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
        let resolver = StaticResolver::new(dynamic_res, static_res);
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
        let repo = VarsRepository::new(full.into_iter());
        assert!(repo.is_ok());
        let missing = vec![VAR_DIRECTORY.clone(), VAR_LISTING.clone()];
        let repo_err = VarsRepository::new(missing.into_iter());
        assert!(repo_err.is_err());
        assert_eq!(
            repo_err.unwrap_err(),
            ErrorsVarsRepository::MissingDependencies(Identifiers(vec![VAR_PATTERN_NAME.clone()]))
        );
    }

    #[test]
    fn test_var_repository_execution_sequence() {
        let full = vec![
            VAR_DIRECTORY.clone(),
            VAR_LISTING.clone(),
            VAR_PATTERN.clone(),
        ];
        let repo = VarsRepository::new(full.into_iter()).unwrap();
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
        let resolver = StaticResolver::new(dynamic_res, static_res);
        let full = vec![
            VAR_DIRECTORY.clone(),
            VAR_LISTING.clone(),
            VAR_PATTERN.clone(),
        ];
        let repo = VarsRepository::new(full.into_iter()).unwrap();
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
