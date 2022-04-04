use std::collections::HashMap;

use crate::entities::aliases::{Alias, AliasAndDependencies};
use crate::entities::choices::Choice;
use crate::entities::dependencies::ErrorsDependencies;
use crate::entities::identifiers::Identifier;
use crate::entities::processes::ShellCommand;
use crate::entities::vars::Var;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct ResolverContext {
    pub alias: Alias,
    pub full_name: String,
    pub choices: HashMap<Identifier, Vec<Choice>>,
    pub execution_sequence: Vec<Identifier>,
}

pub trait Resolver {
    fn resolve_input(
        &self,
        var: &Var,
        prompt: &str,
        ctx: &ResolverContext,
    ) -> Result<Choice, ErrorsResolver>;
    // TODO make cmd a string
    fn resolve_dynamic<CMD>(
        &self,
        var: &Var,
        cmd: CMD,
        ctx: &ResolverContext,
    ) -> Result<Vec<Choice>, ErrorsResolver>
    where
        CMD: Into<ShellCommand<String>>;
    fn resolve_static(
        &self,
        var: &Var,
        choices: impl Iterator<Item = Choice>,
        ctx: &ResolverContext,
    ) -> Result<Vec<Choice>, ErrorsResolver>;
    fn select_identifier(
        &self,
        identifiers: &[AliasAndDependencies],
        prmpt: &str,
    ) -> Result<AliasAndDependencies, ErrorsResolver>;
}

#[derive(Debug, Error)]
pub enum ErrorsResolver {
    #[error("while performing choices substitution\n{0}")]
    Dependencies(#[from] ErrorsDependencies),
    #[error("no choice is available for var {0}")]
    NoChoiceWasAvailable(Identifier),
    #[error("an error happened when gathering choices for identifier {0}\n-> {1}")]
    DynamicResolveFailure(Identifier, Box<dyn std::error::Error>),
    #[error(
        "gathering choices for {0} failed because the command\n   {}{}{1}{} \n   returned empty content on stdout. stderr content was \n {2}", termion::color::Fg(termion::color::Cyan), termion::style::Bold, termion::style::Reset
    )]
    DynamicResolveEmpty(Identifier, String, String),
    #[error("no choice was selected for var {0}")]
    NoChoiceWasSelected(Identifier),
    #[error("no input for for var {0} because {1}")]
    NoInputWasProvided(Identifier, String),
    #[error("selection empty")]
    IdentifierSelectionEmpty(),
    #[error("selection invalid.")]
    IdentifierSelectionInvalid(Box<dyn std::error::Error>),
}
