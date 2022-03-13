mod dependency_resolution;
pub mod resolver;

pub use dependency_resolution::choice_for_var;
pub use dependency_resolution::choices_for_execution_sequence;
pub use dependency_resolution::execution_sequence_for_dependencies;
pub use dependency_resolution::ErrorDependencyResolution;
pub use dependency_resolution::VarsCollection;
pub use dependency_resolution::VarsDefaultValues;

#[cfg(test)]
pub mod mocks {
    use super::dependency_resolution;
    use crate::entities::choices::Choice;
    use crate::entities::identifiers::Identifier;
    use crate::entities::processes::ShellCommand;
    use crate::entities::vars::Var;
    pub use dependency_resolution::mocks::*;
    use std::collections::HashMap;

    use super::resolver::{ErrorsResolver, Resolver};

    #[derive(Debug)]
    pub struct StaticResolver {
        dynamic_res: HashMap<String, Vec<Choice>>,
        static_res: HashMap<Identifier, Vec<Choice>>,
        identifier_to_select: Option<Identifier>,
    }
    impl StaticResolver {
        pub const fn new(
            dynamic_res: HashMap<String, Vec<Choice>>,
            static_res: HashMap<Identifier, Vec<Choice>>,
            identifier_to_select: Option<Identifier>,
        ) -> Self {
            StaticResolver {
                dynamic_res,
                static_res,
                identifier_to_select,
            }
        }
    }
    impl Resolver for StaticResolver {
        fn resolve_input(&self, var: &Var, _: &str) -> Result<Choice, ErrorsResolver> {
            self.static_res
                .get(&var.name())
                .and_then(|e| e.first())
                .map(|e| e.to_owned())
                .ok_or(ErrorsResolver::NoChoiceWasAvailable(var.name()))
        }

        fn resolve_dynamic<CMD>(
            &self,
            var: &Var,
            cmds: Vec<CMD>,
        ) -> Result<Vec<Choice>, ErrorsResolver>
        where
            CMD: Into<ShellCommand<String>>,
        {
            let choices: Vec<Choice> = cmds
                .into_iter()
                .flat_map(|cmd| {
                    let sh_cmd = Into::<ShellCommand<String>>::into(cmd);
                    let query = sh_cmd.value();
                    self.dynamic_res
                        .iter()
                        .find(|(key, _)| *key == query)
                        .and_then(|(_, value)| value.first())
                        .cloned()
                })
                .collect();

            if choices.is_empty() {
                Err(ErrorsResolver::NoChoiceWasAvailable(var.name()))
            } else {
                Ok(choices)
            }
        }

        fn resolve_static(
            &self,
            var: &Var,
            _cmd: impl Iterator<Item = Choice>,
        ) -> Result<Vec<Choice>, ErrorsResolver> {
            self.static_res
                .get(&var.name())
                .map(|c| c.to_owned())
                .ok_or(ErrorsResolver::NoChoiceWasSelected(var.name()))
        }
        fn select_identifier(
            &self,
            _: &[Identifier],
            _: Option<&[&str]>,
            _: &str,
        ) -> Result<Identifier, ErrorsResolver> {
            self.identifier_to_select
                .clone()
                .ok_or(ErrorsResolver::IdentifierSelectionEmpty())
        }
    }
}
