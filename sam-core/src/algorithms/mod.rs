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
    use super::{dependency_resolution, resolver::ResolverContext};
    use crate::entities::identifiers::Identifier;
    use crate::entities::vars::Var;
    use crate::entities::{aliases::AliasAndDependencies, choices::Choice};
    pub use dependency_resolution::mocks::*;
    use std::collections::HashMap;

    use super::resolver::{ErrorsResolver, Resolver};

    #[derive(Debug)]
    pub struct StaticResolver {
        identifier_to_select: Option<Identifier>,
        dynamic_res: HashMap<String, Vec<Choice>>,
        static_res: HashMap<Identifier, Vec<Choice>>,
    }
    impl StaticResolver {
        pub const fn new(
            identifier_to_select: Option<Identifier>,
            dynamic_res: HashMap<String, Vec<Choice>>,
            static_res: HashMap<Identifier, Vec<Choice>>,
        ) -> Self {
            StaticResolver {
                identifier_to_select,
                dynamic_res,
                static_res,
            }
        }
    }
    impl Resolver for StaticResolver {
        fn resolve_input(
            &self,
            var: &Var,
            _: &str,
            _ctx: &ResolverContext,
        ) -> Result<Choice, ErrorsResolver> {
            self.static_res
                .get(&var.name())
                .and_then(|e| e.first())
                .map(|e| e.to_owned())
                .ok_or_else(|| ErrorsResolver::NoChoiceWasAvailable(var.name()))
        }

        fn resolve_dynamic(
            &self,
            var: &Var,
            cmd: String,
            _ctx: &ResolverContext,
        ) -> Result<Vec<Choice>, ErrorsResolver> {
            let choices = self
                .dynamic_res
                .iter()
                .find(|(key, _)| *key == &cmd)
                .and_then(|(_, value)| value.first())
                .cloned();

            if let Some(choic) = choices {
                Ok(vec![choic])
            } else {
                Err(ErrorsResolver::NoChoiceWasAvailable(var.name()))
            }
        }

        fn resolve_static(
            &self,
            var: &Var,
            _cmd: impl Iterator<Item = Choice>,
            _ctx: &ResolverContext,
        ) -> Result<Vec<Choice>, ErrorsResolver> {
            self.static_res
                .get(&var.name())
                .map(|c| c.to_owned())
                .ok_or_else(|| ErrorsResolver::NoChoiceWasSelected(var.name()))
        }
        fn select_identifier(
            &self,
            aliases: &[AliasAndDependencies],
            _: &str,
        ) -> Result<AliasAndDependencies, ErrorsResolver> {
            if let Some(id_to_select) = &self.identifier_to_select {
                for alias in aliases {
                    if alias.alias.identifier() == id_to_select {
                        return Ok(alias.to_owned());
                    }
                }
            }
            Err(ErrorsResolver::IdentifierSelectionEmpty())
        }
    }
}
