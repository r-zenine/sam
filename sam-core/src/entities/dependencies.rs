use crate::entities::choices::Choice;
use crate::entities::commands::Command;
use crate::entities::identifiers::Identifier;
use regex::Regex;
use std::collections::HashMap;
use thiserror::Error;


pub trait Dependencies: Command {
    fn substitute_for_choices(
        &self,
        choices: &HashMap<Identifier, Vec<Choice>>,
    ) -> Result<Vec<String>, ErrorsDependencies> {
        let mut command = vec![self.command().to_string()];
        for dep in self.dependencies() {
            let mut new_commands = Vec::with_capacity(command.len());
            if let Some(choices_for_dep) = choices.get(&dep) {
                for choice in choices_for_dep {
                    let out = command
                        .iter()
                        .map(|cmd| substitute_choice(cmd, &dep, choice.value()));
                    new_commands.extend(out);
                }
            } else {
                return Err(ErrorsDependencies::MissingChoicesForVar(dep));
            }
            command = new_commands;
        }
        Ok(command)
    }

    fn substitute_for_choices_partial(&self, choices: &HashMap<Identifier, Choice>) -> String {
        let mut command = self.command().to_string();
        for dep in self.dependencies() {
            if let Some(chce) = choices.get(&dep) {
                command = substitute_choice(&command, &dep, chce.value());
            }
        }
        command
    }
}

fn substitute_choice(origin: &str, dependency: &Identifier, choice: &str) -> String {
    let re_fmt = format!(r#"(?P<var>\{{\{{ ?{} ?\}}\}})"#, dependency.name());
    let re2_fmt = format!(
        r#"(?P<var>\{{\{{ ?{}::{} ?\}}\}})"#,
        dependency.namespace.clone().unwrap_or_default(),
        dependency.name()
    );
    let re: Regex = Regex::new(re_fmt.as_str()).unwrap();
    let re2: Regex = Regex::new(re2_fmt.as_str()).unwrap();
    let tmp = re.replace(origin, choice).to_string();
    re2.replace(&tmp, choice).to_string()
}

#[derive(Debug)]
pub struct ExecutionSequence {
    inner: Vec<Identifier>,
}

impl ExecutionSequence {
    pub fn new(inner: Vec<&Identifier>) -> Self {
        ExecutionSequence {
            inner: inner.into_iter().cloned().collect(),
        }
    }
    pub fn identifiers(&self) -> Vec<Identifier> {
        let mut rep: Vec<Identifier> = Vec::with_capacity(self.inner.len());
        for e in self.inner.clone() {
            rep.push(e.clone());
        }
        rep
    }

    pub fn as_slice(&self) -> &[Identifier] {
        self.inner.as_slice()
    }
}

impl AsRef<[Identifier]> for ExecutionSequence {
    fn as_ref(&self) -> &[Identifier] {
        self.inner.as_slice()
    }
}



#[derive(Debug, Error)]
pub enum ErrorsDependencies {
    #[error("no choice is available for var {0}")]
    MissingChoicesForVar(Identifier),
}
