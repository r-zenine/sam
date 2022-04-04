use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet, VecDeque},
};

use crate::entities::{
    aliases::Alias,
    choices::Choice,
    commands::Command,
    dependencies::{Dependencies, ExecutionSequence},
    identifiers::{Identifier, Identifiers},
    processes::ShellCommand,
    vars::Var,
};

use crate::algorithms::resolver::{ErrorsResolver, Resolver};
use thiserror::Error;

use super::resolver::ResolverContext;

pub trait VarsCollection {
    fn get(&self, id: &Identifier) -> Option<&Var>;
}

pub trait VarsDefaultValues {
    fn default_value(&self, id: &Identifier) -> Option<&Choice>;
}

pub fn execution_sequence_for_dependencies<Deps: Dependencies>(
    vars: &dyn VarsCollection,
    dep: Deps,
) -> std::result::Result<ExecutionSequence, ErrorDependencyResolution> {
    let mut already_seen = HashSet::new();
    let mut already_inserted = HashSet::new();
    let mut candidates = dep.dependencies();
    let mut missing = Vec::default();
    let mut execution_seq = VecDeque::default();

    while let Some(cur) = candidates.pop() {
        if already_seen.contains(&cur) && !already_inserted.contains(&cur) {
            already_inserted.insert(cur.clone());
            if let Some(cur_var) = vars.get(&cur) {
                execution_seq.push_back(Borrow::borrow(cur_var));
            }
            continue;
        }
        if already_seen.contains(&cur) {
            continue;
        }
        if let Some(cur_var) = vars.get(&cur) {
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
        Err(ErrorDependencyResolution::MissingDependencies(Identifiers(
            missing,
        )))
    } else {
        Ok(ExecutionSequence::new(execution_seq.into_iter().collect()))
    }
}

#[derive(Debug, Error)]
pub enum ErrorDependencyResolution {
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

pub fn choices_for_execution_sequence<R: Resolver>(
    alias: &Alias,
    vars_col: &dyn VarsCollection,
    vars_defaults: &dyn VarsDefaultValues,
    resolver: &R,
    vars: ExecutionSequence,
) -> std::result::Result<Vec<(Identifier, Vec<Choice>)>, ErrorDependencyResolution> {
    let mut ctx = ResolverContext {
        alias: alias.clone(),
        full_name: alias.full_name().to_string(),
        choices: HashMap::new(),
        execution_sequence: vars.identifiers(),
    };
    for var_name in vars.as_slice() {
        if let Some(var) = vars_col.get(var_name) {
            let choice = if let Some(default) = vars_defaults.default_value(&var.name()) {
                vec![default.to_owned()]
            } else {
                choice_for_var(resolver, var, &ctx.choices, &ctx)?
            };
            ctx.choices.insert(var.name(), choice);
        } else {
            return Err(ErrorDependencyResolution::MissingDependencies(Identifiers(
                vec![(*var_name).clone()],
            )));
        }
    }
    Ok(ctx.choices.into_iter().collect())
}

/// will return a valid choice for the current Var using the provided VarResolver and the
/// HashMap of choices provided.
/// First, this function will look into the `choices` HashMap to fill values for all the dependencies of the current
/// `Var`and then use the resolver to get a `Choice` for the current `Var`
pub fn choice_for_var<'repository, R>(
    resolver: &'repository R,
    var: &'repository Var,
    choices: &'repository HashMap<Identifier, Vec<Choice>>,
    ctx: &ResolverContext,
) -> std::result::Result<Vec<Choice>, ErrorDependencyResolution>
where
    R: Resolver,
{
    resolve_choice_for_var(resolver, var, choices, ctx).map_err(|err| {
        ErrorDependencyResolution::NoChoiceForVar {
            var_name: var.name(),
            error: err,
        }
    })
}

fn resolve_choice_for_var<'repository, R>(
    resolver: &'repository R,
    var: &'repository Var,
    choices: &'repository HashMap<Identifier, Vec<Choice>>,
    ctx: &ResolverContext,
) -> std::result::Result<Vec<Choice>, ErrorsResolver>
where
    R: Resolver,
{
    if var.is_command() {
        let mut choices_out: Vec<Choice> = vec![];
        let mut has_one_rep = true;
        let commands: Vec<ShellCommand<String>> = var
            .substitute_for_choices(choices)?
            .iter()
            .map(Clone::clone)
            .map(ShellCommand::new)
            .collect();
        for command in commands {
            let mut choices = resolver.resolve_dynamic(var, command, ctx)?;
            has_one_rep = has_one_rep & (choices.len() == 1);
            choices_out.append(&mut choices);
        }
        if choices_out.is_empty() {
            // TODO fixme
            Err(ErrorsResolver::DynamicResolveEmpty(
                var.name(),
                String::new(),
                String::new(),
            ))
        } else {
            if has_one_rep {
                Ok(choices_out)
            } else {
                resolver.resolve_static(var, choices_out.into_iter(), ctx)
            }
        }
    } else if var.is_input() {
        let prompt = var.prompt().unwrap_or("no provided prompt");
        resolver.resolve_input(var, prompt, ctx).map(|c| vec![c])
    } else {
        resolver.resolve_static(var, var.choices().into_iter(), ctx)
    }
}

pub mod mocks {
    use std::collections::HashMap;

    use crate::entities::{choices::Choice, identifiers::Identifier, vars::Var};

    use super::{VarsCollection, VarsDefaultValues};

    #[derive(Default)]
    pub struct VarsDefaultValuesMock(pub HashMap<Identifier, Vec<Choice>>);
    #[derive(Default)]
    pub struct VarsCollectionMock(pub HashMap<Identifier, Var>);

    impl VarsCollection for VarsCollectionMock {
        fn get(&self, id: &Identifier) -> Option<&Var> {
            self.0.get(id)
        }
    }
    impl VarsDefaultValues for VarsDefaultValuesMock {
        fn default_value(&self, id: &Identifier) -> Option<&Choice> {
            self.0.get(id).and_then(|v| v.first())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::algorithms::dependency_resolution::mocks::{
        VarsCollectionMock, VarsDefaultValuesMock,
    };
    use crate::algorithms::dependency_resolution::resolve_choice_for_var;
    use crate::algorithms::mocks::StaticResolver;
    use crate::algorithms::resolver::ResolverContext;
    use crate::algorithms::{choices_for_execution_sequence, execution_sequence_for_dependencies};
    use crate::entities::choices::Choice;
    use crate::entities::identifiers::fixtures::*;
    use crate::entities::vars::fixtures::*;
    use maplit::hashmap;

    #[test]
    fn test_resolve() {
        let choices = hashmap! {
            VAR_DIRECTORY_NAME.clone() => vec![VAR_DIRECTORY_CHOICE_1.clone()],
            VAR_PATTERN_NAME.clone() => vec![VAR_PATTERN_CHOICE_2.clone()],
        };
        let command_final = format!(
            "ls -l {} |grep -v {}",
            VAR_DIRECTORY_CHOICE_1.value(),
            VAR_PATTERN_CHOICE_2.value()
        );
        let choice_final = Choice::from_value("final_value");
        let dynamic_res = hashmap![
            command_final => vec![choice_final.clone()],
        ];
        let static_res = hashmap![
            VAR_DIRECTORY_NAME.clone() => vec![VAR_DIRECTORY_CHOICE_1.clone()],
            VAR_PATTERN_NAME.clone() => vec![VAR_PATTERN_CHOICE_2.clone()],
        ];
        let var1 = VAR_LISTING.clone();
        let alias = crate::entities::aliases::fixtures::ALIAS_GREP_DIR.clone();
        let resolver = StaticResolver::new(Some(alias.identifier()), dynamic_res, static_res);
        let ctx = ResolverContext {
            alias: alias.clone(),
            full_name: alias.full_name().to_string(),
            choices: choices.clone(),
            execution_sequence: vec![],
        };
        let ret_var1 = resolve_choice_for_var(&resolver, &var1, &choices, &ctx);
        assert!(ret_var1.is_ok());
        assert_eq!(*ret_var1.unwrap().first().unwrap(), choice_final);
        let var2 = VAR_PATTERN.clone();
        let ret_var2 = resolve_choice_for_var(&resolver, &var2, &choices, &ctx);
        assert!(ret_var2.is_ok());
        assert_eq!(
            *ret_var2.unwrap().first().unwrap(),
            VAR_PATTERN_CHOICE_2.clone()
        );
    }

    #[test]
    fn test_var_repository_execution_sequence() {
        let full = vec![
            VAR_DIRECTORY.clone(),
            VAR_LISTING.clone(),
            VAR_PATTERN.clone(),
        ];
        let repo = VarsCollectionMock(full.into_iter().map(|c| (c.name(), c)).collect());
        let seq = execution_sequence_for_dependencies(&repo, VAR_LISTING.clone());
        assert!(seq.is_ok());
        let seq = execution_sequence_for_dependencies(&repo, VAR_USE_LISTING.clone());
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
            command_final => vec![choice_final.clone()],
        ];
        let static_res = hashmap![
            VAR_DIRECTORY_NAME.clone() => vec![ VAR_DIRECTORY_CHOICE_1.clone()],
            VAR_PATTERN_NAME.clone() => vec![VAR_PATTERN_CHOICE_2.clone()],
        ];
        let resolver = StaticResolver::new(None, dynamic_res, static_res);
        let full = vec![
            VAR_DIRECTORY.clone(),
            VAR_LISTING.clone(),
            VAR_PATTERN.clone(),
        ];
        let repo = VarsCollectionMock(full.into_iter().map(|c| (c.name(), c)).collect());
        let defaults = VarsDefaultValuesMock::default();
        let alias = &crate::entities::aliases::fixtures::ALIAS_GREP_DIR;
        let seq = execution_sequence_for_dependencies(&repo, VAR_USE_LISTING.clone()).unwrap();
        let res = choices_for_execution_sequence(alias, &repo, &defaults, &resolver, seq);
        assert!(res.is_ok());
        let mut expected = vec![
            (VAR_PATTERN_NAME.clone(), vec![VAR_PATTERN_CHOICE_2.clone()]),
            (VAR_LISTING_NAME.clone(), vec![choice_final]),
            (
                VAR_DIRECTORY_NAME.clone(),
                vec![VAR_DIRECTORY_CHOICE_1.clone()],
            ),
        ];
        let mut result = res.unwrap();
        result.sort();
        expected.sort();
        assert_eq!(result, expected);
    }
}
