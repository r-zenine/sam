use crate::aliases::{Alias, ResolvedAlias};
use crate::choices::Choice;
use crate::commands::Command;
use crate::dependencies::{ErrorsResolver, Resolver};
use crate::identifiers::Identifier;
use crate::repositories::{AliasesRepository, ErrorsAliasesRepository};
use crate::repositories::{ErrorsVarsRepository, VarsRepository};
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;
use thiserror::Error;

const PROMPT: &str = "Choose an alias to run > ";

#[derive(Clone, Debug, PartialEq)]
pub enum SamCommand {
    ChooseAndExecuteAlias,
    ExecuteAlias { alias: Identifier },
    DisplayLastExecutedAlias,
    ExecuteLastExecutedAlias,
    ModifyThenExecuteLastAlias,
    DisplayHistory,
}

pub struct SamEngine<R: Resolver> {
    pub resolver: R,
    pub aliases: AliasesRepository,
    pub vars: VarsRepository,
    pub logger: Rc<dyn SamLogger>,
    pub history: Box<dyn SamHistory>,
    pub env_variables: HashMap<String, String>,
    pub executor: Rc<dyn SamExecutor>,
}

impl<R: Resolver> SamEngine<R> {
    pub fn run(&mut self, command: SamCommand) -> Result<i32> {
        use SamCommand::*;
        match command {
            ChooseAndExecuteAlias => self.choose_and_execute_alias(),
            ExecuteAlias { alias } => self.execute_alias(&alias),
            DisplayLastExecutedAlias => self.display_last_executed_alias(),
            ExecuteLastExecutedAlias => self.execute_last_executed_alias(),
            ModifyThenExecuteLastAlias => self.modify_then_execute_last_executed_alias(),
            DisplayHistory => self.display_history(),
        }
    }

    fn choose_and_execute_alias(&self) -> Result<i32> {
        let id = self.aliases.select_alias(&self.resolver, PROMPT)?;
        self.run_alias(id)
    }

    fn execute_alias(&self, alias_id: &Identifier) -> Result<i32> {
        let alias = self.aliases.get(alias_id)?;
        self.run_alias(alias)
    }

    fn run_alias(&self, alias: &Alias) -> Result<i32> {
        let exec_seq = self.vars.execution_sequence(alias)?;
        let choices: HashMap<Identifier, Choice> = self
            .vars
            .choices(&self.resolver, exec_seq)?
            .into_iter()
            .collect();

        let final_alias = alias.with_choices(&choices).unwrap();
        self.history.put(final_alias.clone())?;
        self.logger.final_command(alias, &final_alias.command());
        self.executor
            .execute_resolved_alias(&final_alias, &self.env_variables)
    }

    fn display_last_executed_alias(&self) -> Result<i32> {
        let resolved_alias_o = self.history.get_last()?;
        if let Some(alias) = resolved_alias_o {
            println!("Alias: {}", &alias.name());
            println!("{}", &alias.command());
        }
        Ok(0)
    }

    fn display_history(&self) -> Result<i32> {
        let resolved_alias_o = self.history.get_last_n(10)?;
        for alias in resolved_alias_o {
            println!("\n=============\n");
            print!("{}", alias);
            print!("\n=============\n");
        }
        Ok(0)
    }

    fn modify_then_execute_last_executed_alias(&mut self) -> Result<i32> {
        let resolved_alias_o = self.history.get_last()?;
        if let Some(resolved_alias) = resolved_alias_o {
            let original_alias = Alias::from(resolved_alias.clone());
            let exec_seq = self.vars.execution_sequence(original_alias.clone())?;
            let identifiers = exec_seq.identifiers();
            if !identifiers.is_empty() {
                let selected_var = self.resolver.select_identifier(
                    &identifiers,
                    None,
                    "Select the variable to override:",
                )?;

                let var_position = identifiers
                    .iter()
                    .position(|x| x == &selected_var)
                    .unwrap_or_default();

                let new_defaults: HashMap<Identifier, Choice> = identifiers
                    .into_iter()
                    .skip(var_position + 1)
                    .flat_map(|e| resolved_alias.choice(&e).map(|choice| (e, choice)))
                    .collect();

                self.vars.set_defaults(&new_defaults)?;
            }
            self.execute_alias(&original_alias.identifier())
        } else {
            println!("history empty");
            Ok(0)
        }
    }

    fn execute_last_executed_alias(&self) -> Result<i32> {
        let resolved_alias_o = self.history.get_last()?;
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
    fn put(&self, alias: ResolvedAlias) -> Result<()>;
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
    VarsRepository(#[from] ErrorsVarsRepository),
    #[error("could not select the alias to run\n-> {0}")]
    AliasRepository(#[from] ErrorsAliasesRepository),
    #[error("could not run a command\n-> {0}")]
    SubCommand(#[from] std::io::Error),
    #[error("history is unavailable\n-> {0}")]
    HistoryNotAvailable(#[from] Box<dyn std::error::Error>),
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, rc::Rc};

    use crate::{choices::Choice, dependencies::mocks::StaticResolver, identifiers::Identifier};
    use maplit::hashmap;

    use crate::engines::mocks::{InMemoryHistory, LogExecutor, SilentLogger};

    use crate::engines::{SamCommand, SamEngine};

    use super::{fixtures, SamExecutor};

    #[test]
    fn choose_and_execute_alias() {
        let variable_1 = Identifier::new("variable_1");
        let variable_2 = Identifier::new("variable_2");
        let choice_v_1 = Choice::new("value_1", None);
        let choice_v_2 = Choice::new("toto", None);

        let static_res = hashmap! {
            variable_1.clone() => choice_v_1.clone(),
        };
        let dynamic_res = hashmap! {
            String::from("echo '$SOME_ENV_VAR\\ntoto'") => Choice::new("toto", None)
        };

        let executor = Rc::new(LogExecutor::default());
        let selected_identifier = Identifier::new("alias_1");
        let mut engine = make_engine(
            dynamic_res,
            static_res,
            Some(selected_identifier.clone()),
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
        assert_eq!(resolved_alias.choices().len(), 2);
        assert_eq!(resolved_alias.choice(&variable_1).unwrap(), choice_v_1);
        assert_eq!(resolved_alias.choice(&variable_2).unwrap(), choice_v_2);
        assert_eq!(&engine.history.get_last().unwrap().unwrap(), resolved_alias);
    }

    #[test]
    fn execute_alias() {
        let chosen_alias = Identifier::new("alias_2");
        let variable_1 = Identifier::new("variable_1");
        let variable_2 = Identifier::new("variable_2");
        let choice_v_1 = Choice::new("value_1", None);
        let choice_v_2 = Choice::new("toto", None);

        let static_res = hashmap! {
            variable_1.clone() => choice_v_1.clone(),
        };
        let dynamic_res = hashmap! {
            String::from("echo '$SOME_ENV_VAR\\ntoto'") => Choice::new("toto", None)
        };

        let executor = Rc::new(LogExecutor::default());
        let mut engine = make_engine(dynamic_res, static_res, None, executor.clone());
        engine
            .run(SamCommand::ExecuteAlias {
                alias: chosen_alias,
            })
            .expect("Should not return an error");
        let resolved_aliases = executor.commands.borrow();

        // Only one alias was executed
        assert_eq!(resolved_aliases.len(), 1);
        let (resolved_alias, _env_vars) = resolved_aliases.first().unwrap();
        assert!(resolved_alias.choice(&variable_1).is_some());
        assert_eq!(resolved_alias.choices().len(), 2);
        assert_eq!(resolved_alias.choice(&variable_1).unwrap(), choice_v_1);
        assert_eq!(resolved_alias.choice(&variable_2).unwrap(), choice_v_2);
        assert_eq!(&engine.history.get_last().unwrap().unwrap(), resolved_alias);
    }

    fn make_engine(
        dynamic_res: HashMap<String, Choice>,
        static_res: HashMap<Identifier, Choice>,
        selected_identifier: Option<Identifier>,
        executor: Rc<dyn SamExecutor>,
    ) -> SamEngine<StaticResolver> {
        let history = Box::new(InMemoryHistory::default());
        let logger = Rc::new(SilentLogger {});
        let sam_data = fixtures::multi_namespace_aliases_and_vars();
        let resolver = StaticResolver::new(dynamic_res, static_res, selected_identifier);
        SamEngine {
            resolver,
            aliases: sam_data.aliases,
            vars: sam_data.vars,
            logger,
            history,
            env_variables: sam_data.env_variables,
            executor,
        }
    }
}

#[cfg(test)]
mod mocks {
    use std::cell::RefCell;

    use crate::aliases::ResolvedAlias;

    use super::SamHistory;

    #[derive(Default)]
    pub struct InMemoryHistory {
        pub aliases: RefCell<std::collections::VecDeque<ResolvedAlias>>,
    }

    impl SamHistory for InMemoryHistory {
        fn put(&self, alias: ResolvedAlias) -> super::Result<()> {
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
}

#[cfg(test)]
mod fixtures {
    use std::collections::HashMap;

    use crate::{
        aliases::Alias, repositories::AliasesRepository, repositories::VarsRepository, vars::Var,
    };
    use maplit::hashmap;

    pub struct SamData {
        pub aliases: AliasesRepository,
        pub vars: VarsRepository,
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
              alias: '[[alias_1]] | echo {{variable_1}} '";

        let env_variables = hashmap! {
            "SOME_ENV_VAR".to_string() => "env_var_value".to_string(),
            "SOME_ENV_VAR_2".to_string() => "env_var_value_2".to_string(),
        };

        let vars_r: Vec<Var> = serde_yaml::from_str(vars_str).expect("Test fixtures are bad");
        let aliases_r: Vec<Alias> = serde_yaml::from_str(alias_str).expect("text fixtures are bad");

        let vars = VarsRepository::new(vars_r.into_iter());
        let aliases = AliasesRepository::new(aliases_r.into_iter()).expect("text fixtures are bad");
        SamData {
            aliases,
            vars,
            env_variables,
        }
    }
}
