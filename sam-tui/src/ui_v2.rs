use sam_core::algorithms::resolver::ErrorsResolver;
use sam_core::algorithms::resolver::Resolver;
use sam_core::algorithms::resolver::ResolverContext;

use sam_core::entities::aliases::AliasAndDependencies;
use sam_core::entities::choices::Choice;
use sam_core::entities::commands::Command;
use sam_core::entities::vars::Var;
use sam_readers::read_choices;
use sam_terminals::processes::ShellCommand;
use sam_utils::fsutils::ErrorsFS;
use std::collections::{HashMap, HashSet};

use thiserror::Error;

use sam_persistence::VarsCache;

use crate::modal_view::{ModalView, Value};

pub struct UserInterfaceV2 {
    env_variables: HashMap<String, String>,
    cache: Box<dyn VarsCache>,
}

impl<'a> UserInterfaceV2 {
    pub fn new(variables: HashMap<String, String>, cache: Box<dyn VarsCache>) -> UserInterfaceV2 {
        UserInterfaceV2 {
            env_variables: variables,
            cache,
        }
    }

    pub fn choose<T: Value>(
        &self,
        choices: Vec<T>,
        _prompt: &str,
        allow_multiple: bool,
    ) -> Result<HashSet<T>, ErrorsUIV2> {
        let controller = ModalView::new(choices, vec![], allow_multiple);
        let output = controller.run();
        output
            .map(|e| e.marked_values)
            .ok_or(ErrorsUIV2::EmptySelection)
    }
}
#[derive(Debug, Error)]
pub enum ErrorsUIV2 {
    #[error("no selection was provided")]
    CallsResolveWithUndefinedAlias,
    #[error("tried to choose with undifined alias")]
    EmptySelection,
    #[error("initialisation of UI failed")]
    InitError(Box<dyn std::error::Error>),
    #[error("an unexpected error happend while filling the preview window {0}")]
    IOError(#[from] std::io::Error),
    #[error("an unexpected error happend while initialising the preview window {0}")]
    FSError(#[from] ErrorsFS),
}

impl<'a> Resolver for UserInterfaceV2 {
    fn resolve_input(
        &self,
        var: &Var,
        prompt: &str,
        _ctx: &ResolverContext,
    ) -> Result<Choice, ErrorsResolver> {
        let mut buffer = String::new();
        println!(
            "Please provide an input for variable {}.\n{} :",
            &var.name(),
            prompt
        );
        match std::io::stdin().read_line(&mut buffer) {
            Ok(_) => Ok(Choice::new(buffer.replace('\n', ""), None)),
            Err(err) => Err(ErrorsResolver::NoInputWasProvided(
                var.name(),
                err.to_string(),
            )),
        }
    }

    fn resolve_dynamic(
        &self,
        var: &Var,
        cmd: String,
        _ctx: &ResolverContext,
    ) -> Result<Vec<Choice>, ErrorsResolver> {
        let sh_cmd: ShellCommand<String> = cmd.into();
        let cmd_key = sh_cmd
            .replace_env_vars_in_command(&self.env_variables)
            .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.name(), Box::new(e)))?;
        let cache_entry = self.cache.get(cmd_key.value());
        let (stdout_output, _) = if let Ok(Some(out)) = cache_entry {
            (out.as_bytes().to_owned(), vec![])
        } else {
            let mut to_run = ShellCommand::make_command(sh_cmd);
            to_run.envs(&self.env_variables);
            let output = to_run
                .output()
                .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.name(), e.into()))?;
            if output.status.code() == Some(0) && output.stderr.is_empty() {
                self.cache
                    .put(
                        &var.name().to_string(),
                        cmd_key.value(),
                        &String::from_utf8_lossy(output.stdout.as_slice()).to_owned(),
                    )
                    .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.name(), Box::new(e)))?;
            }
            (output.stdout, output.stderr)
        };

        read_choices(stdout_output.as_slice())
            .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.name(), e.into()))
    }

    fn resolve_static<'b>(
        &'b self,
        var: &Var,
        cmd: impl Iterator<Item = Choice>,
        _ctx: &ResolverContext,
    ) -> Result<Vec<Choice>, ErrorsResolver> {
        let choices: Vec<Choice> = cmd.collect();

        if choices.is_empty() {
            return Err(ErrorsResolver::NoChoiceWasAvailable(var.name()));
        }

        if choices.len() == 1 {
            return Ok(choices);
        }

        let choice = {
            let items: Vec<ChoiceElement<'_>> = choices
                .into_iter()
                .map(|choice| ChoiceElement::from(choice, _ctx))
                .collect();
            let prompt = format!("please make a choices for variable: {}", var.name());
            let choice: Vec<Choice> = self
                .choose(items, &prompt, true)
                .map_err(|_e| ErrorsResolver::NoChoiceWasSelected(var.name()))
                .map(|chosen| chosen.into_iter().map(|e| e.choice).collect())?;
            choice
        };
        Ok(choice)
    }

    fn select_identifier<'b>(
        &'b self,
        identifiers: &[AliasAndDependencies],
        prompt: &str,
    ) -> Result<AliasAndDependencies, ErrorsResolver> {
        let items: Vec<AliasElement> = identifiers
            .iter()
            .map(|identifier| AliasElement(identifier.clone()))
            .collect();
        let alias = self
            .choose(items, prompt, false)
            .map_err(|e| ErrorsResolver::IdentifierSelectionInvalid(Box::new(e)))?
            .iter()
            .next()
            .map(|ae| ae.0.clone());
        alias.ok_or(ErrorsResolver::IdentifierSelectionEmpty())
    }
}

#[derive(Clone, Debug)]
struct AliasElement(AliasAndDependencies);

impl PartialEq for AliasElement {
    fn eq(&self, other: &Self) -> bool {
        self.0.alias == other.0.alias
    }
}

impl Eq for AliasElement {}
impl std::hash::Hash for AliasElement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.alias.full_name().hash(state)
    }
}
impl Value for AliasElement {
    fn text(&self) -> &str {
        &self.0.full_name
    }

    fn preview(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "Name: {}\n\nDescription:\n{}\n\nAlias:\n\n{}\n",
            self.0.alias.name(),
            self.0.alias.desc(),
            self.0.alias.command(),
        ));

        if !self.0.dependencies.is_empty() {
            output.push_str("\nDependencies:\n");
            for id in &self.0.dependencies {
                output.push_str(&format!("- {}\n", id));
            }
        }

        output
    }
}

#[derive(Clone, Debug)]
struct ChoiceElement<'a> {
    resolver_context: &'a ResolverContext,
    choice: Choice,
    text: String,
}

impl<'a> ChoiceElement<'a> {
    pub fn from(choice: Choice, ctx: &'a ResolverContext) -> Self {
        let text = format!(
            "{}    {}",
            choice.value(),
            choice.desc().unwrap_or_default()
        );
        ChoiceElement {
            resolver_context: ctx,
            choice,
            text,
        }
    }
}

impl<'a> Eq for ChoiceElement<'a> {}
impl<'a> PartialEq for ChoiceElement<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.choice == other.choice
    }
}

impl<'a> Value for ChoiceElement<'a> {
    fn text(&self) -> &str {
        &self.text
    }

    fn preview(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "Name: {}\n\nDescription:\n{}\n\nAlias:\n\n{}\n",
            self.resolver_context.alias.name(),
            self.resolver_context.alias.desc(),
            self.resolver_context.alias.command(),
        ));

        if !self.resolver_context.execution_sequence.is_empty() {
            output.push_str("\nDependencies:\n");
            for id in &self.resolver_context.execution_sequence {
                output.push_str(&format!("- {}\n", id));
            }
        }

        if !self.resolver_context.choices.is_empty() {
            output.push_str("\nCurrent Choices:\n");
            for (id, choice) in self.resolver_context.choices.iter() {
                output.push_str(&format!("- {} = {:?}", id, choice));
            }
        }
        output
    }
}

impl std::hash::Hash for ChoiceElement<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.choice.value().hash(state);
    }
}
