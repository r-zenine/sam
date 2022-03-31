use crate::algorithms::resolver::{ErrorsResolver, Resolver};
use crate::algorithms::{
    choices_for_execution_sequence, execution_sequence_for_dependencies, ErrorDependencyResolution,
    VarsCollection, VarsDefaultValues,
};
use crate::entities::aliases::{Alias, AliasAndDependencies, ResolvedAlias};
use crate::entities::choices::Choice;
use crate::entities::identifiers::Identifier;
use std::cell::RefCell;
// TODO get rid of this import
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;
use thiserror::Error;

const PROMPT: &str = "Choose an alias to run > ";

pub trait VarsDefaultValuesSetter {
    fn set_defaults(&mut self, defaults: &HashMap<Identifier, Vec<Choice>>);
}

pub trait AliasCollection {
    fn select_alias<R: Resolver>(
        &self,
        r: &R,
        vars: &dyn VarsCollection,
        prompt: &str,
    ) -> std::result::Result<&Alias, ErrorsAliasCollection> {
        let mut qualified_aliases = vec![];
        for dep in self.aliases() {
            let exec_seq = execution_sequence_for_dependencies(vars, dep)?;
            let q_alias = AliasAndDependencies {
                alias: dep.clone(),
                full_name: dep.full_name().to_string(),
                dependencies: exec_seq.identifiers(),
            };
            qualified_aliases.push(q_alias);
        }
        let selection = r.select_identifier(&qualified_aliases, prompt)?;
        self.get(&selection.alias.identifier()).ok_or_else(|| {
            ErrorsAliasCollection::AliasInvalidSelection(selection.alias.identifier())
        })
    }

    fn get(&self, id: &Identifier) -> Option<&Alias>;
    fn aliases(&self) -> Vec<&Alias>;
}

#[derive(Debug, Error)]
pub enum ErrorsAliasCollection {
    #[error("Alias selection failed because \n-> {0}")]
    AliasSelectionFailure(#[from] ErrorsResolver),
    #[error("Invalid alias selected {0}")]
    AliasInvalidSelection(Identifier),
    #[error("Can't figure out dependencies for alias")]
    AliasDependencyResolution(#[from] ErrorDependencyResolution),
}

// Changes:
// Rename SamCommand -> UseCaseAliasExec
//
// Exclude
// -> DisplayHistory,           DisplayLastExecutedAlias   from here and move it to the UseCaseAudit
//
// Exclude
// -> ExecuteLastExecutedAlias, ModifyThenExecuteLastAlias from here and move it to the UseCaseExecutionReplay
//
#[derive(Clone, Debug, PartialEq)]
pub enum SamCommand {
    ChooseAndExecuteAlias,
    ExecuteAlias { alias: Identifier },
    // These 4 should be merged into 1
    // single command that will depend on the SamEngine
    DisplayLastExecutedAlias,
    ExecuteLastExecutedAlias,
    ModifyThenExecuteLastAlias,
    DisplayHistory,
}

// TODO Rename to UseCaseAliasExec
pub struct SamEngine<
    R: Resolver,
    AR: AliasCollection,
    VR: VarsCollection,
    DV: VarsDefaultValuesSetter + VarsDefaultValues,
> {
    pub resolver: R,
    pub aliases: AR,
    pub vars: VR,
    pub defaults: DV,
    pub logger: Rc<dyn SamLogger>,
    pub history: RefCell<Box<dyn SamHistory>>,
    // TODO this should be handled elsewhere, most likely in the executor
    pub env_variables: HashMap<String, String>,
    pub executor: Rc<dyn SamExecutor>,
}

impl<
        R: Resolver,
        AR: AliasCollection,
        VR: VarsCollection,
        DV: VarsDefaultValues + VarsDefaultValuesSetter,
    > SamEngine<R, AR, VR, DV>
{
    pub fn run(&mut self, command: SamCommand) -> Result<i32> {
        use SamCommand::*;
        match command {
            ChooseAndExecuteAlias => self.choose_and_execute_alias(),
            ExecuteAlias { alias } => self.execute_alias(&alias),
            DisplayLastExecutedAlias => self.display_last_executed_alias(),
            ExecuteLastExecutedAlias => self.execute_last_executed_alias(),
            // TODO fixme later
            ModifyThenExecuteLastAlias => Ok(1),
            DisplayHistory => self.display_history(),
        }
    }

    fn choose_and_execute_alias(&self) -> Result<i32> {
        let id = self
            .aliases
            .select_alias(&self.resolver, &self.vars, PROMPT)?;
        self.run_alias(id)
    }

    fn execute_alias(&self, alias_id: &Identifier) -> Result<i32> {
        let alias = self
            .aliases
            .get(alias_id)
            .ok_or_else(|| ErrorsAliasCollection::AliasInvalidSelection(alias_id.clone()))?;
        self.run_alias(alias)
    }

    fn run_alias(&self, alias: &Alias) -> Result<i32> {
        let exec_seq = execution_sequence_for_dependencies(&self.vars, alias)?;
        let choices: HashMap<Identifier, Vec<Choice>> = choices_for_execution_sequence(
            alias,
            &self.vars,
            &self.defaults,
            &self.resolver,
            exec_seq,
        )?
        .into_iter()
        .collect();
        let final_alias = alias.with_choices(&choices).unwrap();
        self.history.borrow_mut().put(final_alias.clone())?;
        self.executor
            .execute_resolved_alias(&final_alias, &self.env_variables)
    }

    fn display_last_executed_alias(&self) -> Result<i32> {
        let resolved_alias_o = self.history.borrow().get_last()?;
        if let Some(alias) = resolved_alias_o {
            println!("Alias: {}", &alias.name());
            println!("Commands:\n=========\n");
            for cmd in alias.commands() {
                println!("\t- {}\n", cmd);
            }
        }
        Ok(0)
    }

    fn display_history(&self) -> Result<i32> {
        let resolved_alias_o = self.history.borrow().get_last_n(10)?;
        for alias in resolved_alias_o {
            println!("\n=============\n");
            print!("{}", alias);
            print!("\n=============\n");
        }
        Ok(0)
    }

    fn execute_last_executed_alias(&self) -> Result<i32> {
        let resolved_alias_o = self.history.borrow().get_last()?;
        if let Some(alias) = resolved_alias_o {
            self.executor
                .execute_resolved_alias(&alias, &self.env_variables)
        } else {
            println!("history empty");
            Ok(0)
        }
    }
}

pub trait SamHistory {
    fn put(&mut self, alias: ResolvedAlias) -> Result<()>;
    fn get_last_n(&self, n: usize) -> Result<Vec<ResolvedAlias>>;
    fn get_last(&self) -> Result<Option<ResolvedAlias>> {
        let mut last = self.get_last_n(1)?;
        Ok(last.pop())
    }
}

pub trait SamLogger {
    fn final_command(&self, alias: &Alias, fc: &dyn Display);
    fn command(&self, var: &dyn Display, cmd: &dyn AsRef<str>);
    fn choice(&self, var: &dyn Display, choice: &dyn Display);
    fn alias(&self, alias: &Alias);
}

pub trait SamExecutor {
    fn execute_resolved_alias(
        &self,
        alias: &ResolvedAlias,
        env_variables: &HashMap<String, String>,
    ) -> Result<i32>;
}

pub type Result<T> = std::result::Result<T, ErrorSamEngine>;

#[derive(Debug, Error)]
pub enum ErrorSamEngine {
    #[error("could not return an exit code.")]
    ExitCode,
    #[error("the requested alias was not found")]
    InvalidAliasSelection,
    #[error("could not resolve the dependency because\n-> {0}")]
    Resolver(#[from] ErrorsResolver),
    #[error("could not figure out dependencies\n-> {0}")]
    DependencyResolution(#[from] ErrorDependencyResolution),
    #[error("could not select the alias to run\n-> {0}")]
    AliasRepositoryT(#[from] ErrorsAliasCollection),
    #[error("could not run a command\n-> {0}")]
    SubCommand(#[from] std::io::Error),
    #[error("history is unavailable\n-> {0}")]
    HistoryNotAvailable(#[from] Box<dyn std::error::Error>),
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::{collections::HashMap, rc::Rc};

    use crate::algorithms::mocks::StaticResolver;
    use crate::algorithms::mocks::{VarsCollectionMock, VarsDefaultValuesMock};
    use crate::entities::{choices::Choice, identifiers::Identifier};
    use maplit::hashmap;

    use crate::engines::mocks::{InMemoryHistory, LogExecutor, SilentLogger};

    use crate::engines::{SamCommand, SamEngine};

    use super::mocks::StaticAliasRepository;
    use super::{fixtures, SamExecutor};

    #[test]
    fn choose_and_execute_alias() {
        let variable_1 = Identifier::new("variable_1");
        let variable_2 = Identifier::new("variable_2");
        let choice_v_1 = Choice::new("value_1", None);
        let choice_v_2 = Choice::new("toto", None);

        let static_res = hashmap! {
            variable_1.clone() => vec![choice_v_1.clone()],
        };
        let dynamic_res = hashmap! {
            String::from("echo '$SOME_ENV_VAR\\ntoto'") => vec![Choice::new("toto", None)]
        };

        let executor = Rc::new(LogExecutor::default());
        let selected_identifier = Identifier::new("alias_1");
        let mut engine = make_engine(
            Some(selected_identifier.clone()),
            dynamic_res,
            static_res,
            executor.clone(),
        );
        engine
            .run(SamCommand::ChooseAndExecuteAlias)
            .expect("Should not return an error");
        let resolved_aliases = executor.commands.borrow();

        // Only one alias was executed
        assert_eq!(resolved_aliases.len(), 1);
        let (resolved_alias, _env_vars) = resolved_aliases.first().unwrap();
        assert_eq!(resolved_alias.name(), &selected_identifier);
        assert!(resolved_alias.choice(&variable_1).is_some());

        // TODO fixme
        assert_eq!(resolved_alias.choices().len(), 2);
        assert_eq!(
            *resolved_alias.choice(&variable_1).unwrap().first().unwrap(),
            choice_v_1
        );
        assert_eq!(
            *resolved_alias.choice(&variable_2).unwrap().first().unwrap(),
            choice_v_2
        );
        assert_eq!(
            &engine.history.borrow().get_last().unwrap().unwrap(),
            resolved_alias
        );
    }

    #[test]
    fn execute_alias() {
        let chosen_alias = Identifier::new("alias_2");
        let variable_1 = Identifier::new("variable_1");
        let variable_2 = Identifier::new("variable_2");
        let choice_v_1 = Choice::new("value_1", None);
        let choice_v_2 = Choice::new("toto", None);

        let static_res = hashmap! {
            variable_1.clone() => vec![ choice_v_1.clone()],
        };
        let dynamic_res = hashmap! {
            String::from("echo '$SOME_ENV_VAR\\ntoto'") => vec![Choice::new("toto", None)]
        };

        let executor = Rc::new(LogExecutor::default());
        let mut engine = make_engine(None, dynamic_res, static_res, executor.clone());
        engine
            .run(SamCommand::ExecuteAlias {
                alias: chosen_alias,
            })
            .expect("Should not return an error");
        let resolved_aliases = executor.commands.borrow();

        // Only one alias was executed
        assert_eq!(resolved_aliases.len(), 1);
        let (resolved_alias, _env_vars) = resolved_aliases.first().unwrap();
        let choices_for_var1 = resolved_alias
            .choice(&variable_1)
            .expect("expected to find choices for variable1");
        let choices_for_var2 = resolved_alias
            .choice(&variable_2)
            .expect("expected to find choices for variable1");

        assert!(choices_for_var1.len() == 1);
        assert_eq!(resolved_alias.choices().len(), 2);
        assert_eq!(choices_for_var1[0], choice_v_1);
        assert_eq!(choices_for_var2[0], choice_v_2);
        assert_eq!(
            &engine.history.borrow().get_last().unwrap().unwrap(),
            resolved_alias
        );
    }

    fn make_engine(
        identifier_to_select: Option<Identifier>,
        dynamic_res: HashMap<String, Vec<Choice>>,
        static_res: HashMap<Identifier, Vec<Choice>>,
        executor: Rc<dyn SamExecutor>,
    ) -> SamEngine<StaticResolver, StaticAliasRepository, VarsCollectionMock, VarsDefaultValuesMock>
    {
        let history = RefCell::new(Box::new(InMemoryHistory::default()));
        let logger = Rc::new(SilentLogger {});
        let sam_data = fixtures::multi_namespace_aliases_and_vars();
        let resolver = StaticResolver::new(identifier_to_select, dynamic_res, static_res);
        SamEngine {
            resolver,
            aliases: sam_data.aliases,
            vars: sam_data.vars,
            defaults: sam_data.defaults,
            logger,
            history,
            env_variables: sam_data.env_variables,
            executor,
        }
    }
}

#[cfg(test)]
mod mocks {
    use crate::algorithms::mocks::VarsDefaultValuesMock;
    use crate::engines::AliasCollection;
    use crate::entities::aliases::Alias;
    use crate::entities::aliases::ResolvedAlias;
    use crate::entities::choices::Choice;
    use crate::entities::identifiers::Identifier;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::collections::VecDeque;

    use super::{SamHistory, VarsDefaultValuesSetter};

    impl VarsDefaultValuesSetter for VarsDefaultValuesMock {
        fn set_defaults(&mut self, defaults: &HashMap<Identifier, Vec<Choice>>) {
            for (key, value) in defaults {
                self.0.insert(key.clone(), value.clone());
            }
        }
    }

    #[derive(Default)]
    pub struct InMemoryHistory {
        pub aliases: RefCell<VecDeque<ResolvedAlias>>,
    }

    impl SamHistory for InMemoryHistory {
        fn put(&mut self, alias: ResolvedAlias) -> super::Result<()> {
            let mut queue = self.aliases.borrow_mut();
            queue.push_front(alias);
            Ok(())
        }

        fn get_last_n(&self, n: usize) -> super::Result<Vec<ResolvedAlias>> {
            Ok(self
                .aliases
                .borrow()
                .iter()
                .take(n)
                .map(ToOwned::to_owned)
                .collect())
        }
    }

    pub struct StaticAliasRepository {
        aliases: HashMap<Identifier, Alias>,
    }

    impl StaticAliasRepository {
        pub fn new(aliases: impl Iterator<Item = Alias>) -> Self {
            let mut mp = HashMap::new();
            for alias in aliases {
                let id = alias.identifier();
                mp.insert(id, alias);
            }
            let mut mpf = HashMap::new();
            for (key, alias) in mp.iter() {
                mpf.insert(key.clone(), alias.clone());
            }
            StaticAliasRepository { aliases: mpf }
        }
    }

    impl AliasCollection for StaticAliasRepository {
        fn get(&self, id: &Identifier) -> Option<&Alias> {
            self.aliases.get(id)
        }

        fn aliases(&self) -> Vec<&Alias> {
            self.aliases.values().collect()
        }
    }
}

#[cfg(test)]
mod fixtures {
    use std::collections::HashMap;

    use crate::{
        algorithms::mocks::{VarsCollectionMock, VarsDefaultValuesMock},
        entities::aliases::Alias,
        entities::vars::Var,
    };
    use maplit::hashmap;

    use super::mocks::StaticAliasRepository;

    pub struct SamData {
        pub aliases: StaticAliasRepository,
        pub vars: VarsCollectionMock,
        pub defaults: VarsDefaultValuesMock,
        pub env_variables: HashMap<String, String>,
    }

    pub fn multi_namespace_aliases_and_vars() -> SamData {
        let vars_str = "
            - name: 'variable_1'
              desc: 'description_ns1_v1'
              choices:
              - value: 'value 1'
                desc: val1 description
              - value: 'value 2'
                desc: val1 description
            - name: 'variable_2'
              desc: 'description_2'
              from_command: echo '$SOME_ENV_VAR\\ntoto'
            - name: 'variable_3'
              desc: 'description_ns2_v1'
              choices:
              - value: 'value 1_ns2'
                desc: val1 description ns2
              - value: 'value 2_ns2'
                desc: val1 description ns2
            - name: 'variable_4'
              desc: description_ns2_v2'
              from_input: prompt";

        let alias_str = "
            - name: 'alias_1'
              desc: 'description of alias_1 in ns1'
              alias: 'some_cmd --type=$SOME_ENV_VAR_2 {{variable_1}}|grep {{variable_2}}'
            - name: 'alias_2'
              desc: 'description of alias_1 in ns2'
              alias: 'some_cmd --type=$SOME_ENV_VAR_2 {{variable_1}}|grep {{variable_2}} | echo {{variable_1}} '";

        let env_variables = hashmap! {
            "SOME_ENV_VAR".to_string() => "env_var_value".to_string(),
            "SOME_ENV_VAR_2".to_string() => "env_var_value_2".to_string(),
        };

        let vars_r: Vec<Var> = serde_yaml::from_str(vars_str).expect("Test fixtures are bad");
        let aliases_r: Vec<Alias> = serde_yaml::from_str(alias_str).expect("text fixtures are bad");

        let vars = VarsCollectionMock(vars_r.into_iter().map(|c| (c.name(), c)).collect());
        let defaults = VarsDefaultValuesMock::default();
        let aliases = StaticAliasRepository::new(aliases_r.into_iter());
        SamData {
            aliases,
            vars,
            defaults,
            env_variables,
        }
    }
}
