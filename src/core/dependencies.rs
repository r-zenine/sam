use crate::core::choices::Choice;
use crate::core::commands::Command;
use crate::core::identifiers::Identifier;
use crate::utils::processes::ShellCommand;
use regex::Regex;
use std::collections::HashMap;
use std::error;
use thiserror::Error;

pub trait Dependencies: Command {
    fn substitute_for_choices<'var>(
        &self,
        choices: &'var HashMap<Identifier, Choice>,
    ) -> Result<String, ErrorsResolver> {
        let mut command = self.command().to_string();
        for dep in self.dependencies() {
            if let Some(chce) = choices.get(&dep) {
                command = substitute_choice(&command, &dep, chce.value());
            } else {
                return Err(ErrorsResolver::NoChoiceWasAvailable(dep));
            }
        }
        Ok(command)
    }

    fn substitute_for_choices_partial<'var>(
        &self,
        choices: &'var HashMap<Identifier, Choice>,
    ) -> String {
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

pub trait Resolver {
    fn resolve_dynamic<CMD>(&self, var: Identifier, cmd: CMD) -> Result<Choice, ErrorsResolver>
    where
        CMD: Into<ShellCommand<String>>;
    fn resolve_static(
        &self,
        var: Identifier,
        cmd: impl Iterator<Item = Choice>,
    ) -> Result<Choice, ErrorsResolver>;
}
#[derive(Debug, Error)]
pub enum ErrorsResolver {
    #[error("no choice is available for var {0}")]
    NoChoiceWasAvailable(Identifier),
    #[error("an error happened when gathering choices for identifier {0}\n-> {1}")]
    DynamicResolveFailure(Identifier, Box<dyn error::Error>),
    #[error(
        "gathering choices for {0} failed because the command\n   {}{}{1}{} \n   returned empty content on stdout. stderr content was \n {2}", termion::color::Fg(termion::color::Cyan), termion::style::Bold, termion::style::Reset
    )]
    DynamicResolveEmpty(Identifier, String, String),
    #[error("no choice was selected for var {0}")]
    NoChoiceWasSelected(Identifier),
}

pub mod mocks {
    use super::{ErrorsResolver, Resolver};
    use crate::core::choices::Choice;
    use crate::core::identifiers::Identifier;
    use crate::utils::processes::ShellCommand;
    use std::collections::HashMap;

    #[derive(Debug)]
    pub struct StaticResolver {
        dynamic_res: HashMap<String, Choice>,
        static_res: HashMap<Identifier, Choice>,
    }
    impl StaticResolver {
        pub fn new(
            dynamic_res: HashMap<String, Choice>,
            static_res: HashMap<Identifier, Choice>,
        ) -> Self {
            StaticResolver {
                dynamic_res,
                static_res,
            }
        }
    }
    impl Resolver for StaticResolver {
        fn resolve_dynamic<CMD>(&self, var: Identifier, cmd: CMD) -> Result<Choice, ErrorsResolver>
        where
            CMD: Into<ShellCommand<String>>,
        {
            let sh_cmd = Into::<ShellCommand<String>>::into(cmd);
            let query = sh_cmd.value();
            self.dynamic_res
                .get(query)
                .map(|e| e.to_owned())
                .ok_or(ErrorsResolver::NoChoiceWasAvailable(var))
        }
        fn resolve_static(
            &self,
            var: Identifier,
            _cmd: impl Iterator<Item = Choice>,
        ) -> Result<Choice, ErrorsResolver> {
            self.static_res
                .get(&var)
                .map(|c| c.to_owned())
                .ok_or(ErrorsResolver::NoChoiceWasSelected(var))
        }
    }
}
