use crate::entities::aliases::{Alias, ResolvedAlias};
use std::fmt::Display;
use std::{cell::RefCell, collections::HashMap};

use crate::engines::{ErrorSamEngine, SamExecutor, SamHistory, SamLogger};

pub struct SilentLogger;
impl SamLogger for SilentLogger {
    fn final_command(&self, _: &Alias, _: &dyn Display) {}
    fn command(&self, _: &dyn Display, _: &dyn AsRef<str>) {}
    fn choice(&self, _: &dyn Display, _: &dyn Display) {}
    fn alias(&self, _: &Alias) {}
}
#[derive(Default)]
pub struct LogExecutor {
    pub commands: RefCell<Vec<(ResolvedAlias, HashMap<String, String>)>>,
}

impl SamExecutor for LogExecutor {
    fn execute_resolved_alias(
        &self,
        alias: &ResolvedAlias,
        env_variables: &HashMap<String, String>,
    ) -> Result<i32, ErrorSamEngine> {
        let mut cmd_mut = self.commands.borrow_mut();
        cmd_mut.push((alias.clone(), env_variables.to_owned()));
        Ok(0)
    }
}

#[derive(Default)]
pub struct InMemoryHistory {
    pub aliases: RefCell<std::collections::VecDeque<ResolvedAlias>>,
}

impl SamHistory for InMemoryHistory {
    fn put(&self, alias: ResolvedAlias) -> Result<(), ErrorSamEngine> {
        let mut queue = self.aliases.borrow_mut();
        queue.push_front(alias);
        Ok(())
    }

    fn get_last_n(&self, n: usize) -> Result<Vec<ResolvedAlias>, ErrorSamEngine> {
        Ok(self
            .aliases
            .borrow()
            .iter()
            .take(n)
            .map(ToOwned::to_owned)
            .collect())
    }
}
