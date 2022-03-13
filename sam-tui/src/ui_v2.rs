use sam_core::algorithms::execution_sequence_for_dependencies;
use sam_core::engines::AliasCollection;
use sam_core::entities::aliases::Alias;
use sam_core::entities::choices::Choice;
use sam_core::entities::commands::Command;
use sam_core::entities::dependencies::{ErrorsResolver, Resolver};
use sam_core::entities::identifiers::Identifier;
use sam_core::entities::processes::ShellCommand;
use sam_core::entities::vars::Var;
use sam_persistence::repositories::{AliasesRepository, VarsRepository};
use sam_readers::read_choices;
use sam_utils::fsutils::ErrorsFS;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use thiserror::Error;

use sam_persistence::VarsCache;

use crate::modal_view::{ModalView, Value};

pub struct UserInterfaceV2 {
    selected_alias: RefCell<Option<Alias>>,
    choices: RefCell<HashMap<Identifier, Vec<Choice>>>,

    alias_repository: AliasesRepository,
    vars_repository: VarsRepository,
    execution_sequence: Vec<Identifier>,
    env_variables: HashMap<String, String>,
    cache: Box<dyn VarsCache>,
}

impl<'a> UserInterfaceV2 {
    pub fn new(
        alias_repository: AliasesRepository,
        vars_repository: VarsRepository,
        variables: HashMap<String, String>,
        cache: Box<dyn VarsCache>,
    ) -> UserInterfaceV2 {
        UserInterfaceV2 {
            selected_alias: RefCell::default(),
            execution_sequence: Vec::default(),
            choices: RefCell::default(),
            env_variables: variables,
            cache,
            alias_repository,
            vars_repository,
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
    fn resolve_input(&self, var: &Var, prompt: &str) -> Result<Choice, ErrorsResolver> {
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

    fn resolve_dynamic<CMD>(&self, var: &Var, cmd: Vec<CMD>) -> Result<Vec<Choice>, ErrorsResolver>
    where
        CMD: Into<ShellCommand<String>>,
    {
        let mut choices_out = vec![];
        for cm in cmd {
            let sh_cmd: ShellCommand<String> = cm.into();
            let cmd_key = sh_cmd
                .replace_env_vars_in_command(&self.env_variables)
                .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.name(), Box::new(e)))?;
            let cache_entry = self.cache.get(cmd_key.value());
            let (stdout_output, _) = if let Ok(Some(out)) = cache_entry {
                (out.as_bytes().to_owned(), vec![])
            } else {
                let mut to_run = ShellCommand::make_command(sh_cmd.clone());
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
                        .map_err(|e| {
                            ErrorsResolver::DynamicResolveFailure(var.name(), Box::new(e))
                        })?;
                }
                (output.stdout, output.stderr)
            };

            let choices = read_choices(stdout_output.as_slice())
                .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.name(), e.into()))?;
            choices_out.extend(choices);
        }
        if choices_out.is_empty() {
            // TODO fixme
            Err(ErrorsResolver::DynamicResolveEmpty(
                var.name(),
                String::new(),
                String::new(),
            ))
        } else {
            self.resolve_static(var, choices_out.into_iter())
        }
    }

    fn resolve_static<'b>(
        &'b self,
        var: &Var,
        cmd: impl Iterator<Item = Choice>,
    ) -> Result<Vec<Choice>, ErrorsResolver> {
        let choices: Vec<Choice> = cmd.collect();

        if choices.is_empty() {
            return Err(ErrorsResolver::NoChoiceWasAvailable(var.name()));
        }

        if choices.len() == 1 {
            return Ok(choices);
        }

        let alias_o = self.selected_alias.clone().take();
        if let Some(alias) = alias_o {
            let choice = {
                let choices_ref = &self.choices.borrow();

                let items: Vec<ChoiceElement<'_>> = choices
                    .into_iter()
                    .map(|choice| {
                        ChoiceElement::from(choice, &alias, choices_ref, &self.execution_sequence)
                    })
                    .collect();
                // TODO fix prompt
                let prompt = format!("please make a choices for variable:\t{}", var.name());
                let choice: Vec<Choice> = self
                    .choose(items, &prompt, false)
                    .map_err(|_e| ErrorsResolver::NoChoiceWasSelected(var.name()))
                    .map(|chosen| chosen.into_iter().map(|e| e.choice).collect())?;
                choice
            };
            let mut mp = self.choices.borrow_mut();
            (*mp).insert(var.name(), choice.clone());
            Ok(choice)
        } else {
            Err(ErrorsResolver::IdentifierSelectionInvalid(Box::new(
                ErrorsUIV2::CallsResolveWithUndefinedAlias,
            )))
        }
    }

    fn select_identifier<'b>(
        &'b self,
        identifiers: &[Identifier],
        _descriptions: Option<&[&str]>,
        prompt: &str,
    ) -> Result<Identifier, ErrorsResolver> {
        let choices_clone = self.choices.borrow();
        let items: Vec<AliasElement<'_>> = identifiers
            .iter()
            .flat_map(|identifier| self.alias_repository.get(identifier).ok())
            .map(|alias| AliasElement::from_alias(alias, &choices_clone, &self.vars_repository))
            .collect();
        let alias = self
            .choose(items, prompt, false)
            .map_err(|e| ErrorsResolver::IdentifierSelectionInvalid(Box::new(e)))?
            .iter()
            .next()
            .map(|ae| ae.alias.clone());
        self.selected_alias.replace(alias.clone());
        alias
            .map(|a| a.identifier())
            .ok_or(ErrorsResolver::IdentifierSelectionEmpty())
    }
}

#[derive(Clone, Debug)]
struct AliasElement<'a> {
    alias: &'a Alias,
    full_name: Cow<'a, str>,
    choices: &'a HashMap<Identifier, Vec<Choice>>,
    execution_sequence: Vec<Identifier>,
}

impl<'a> PartialEq for AliasElement<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.alias == other.alias
    }
}

impl<'a> Eq for AliasElement<'a> {}
impl std::hash::Hash for AliasElement<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.alias.full_name().hash(state)
    }
}
impl<'a> Value for AliasElement<'a> {
    fn text(&self) -> &str {
        &self.full_name
    }

    fn preview(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "Name:\t{}\n\nDescription:\n{}\n\nAlias:\n\n{}\n",
            self.alias.name(),
            self.alias.desc(),
            self.alias.command(),
        ));

        if !self.execution_sequence.is_empty() {
            output.push_str("\nDependencies:\n");
            for id in &self.execution_sequence {
                output.push_str(&format!("- {}\n", id));
            }
        }

        if !self.choices.is_empty() {
            output.push_str("\nCurrent Choices:\n");
            for (id, choice) in self.choices.iter() {
                output.push_str(&format!("- {}\t= {:?}", id, choice));
            }
        }
        output
    }
}

impl<'a> AliasElement<'a> {
    fn from_alias(
        alias: &'a Alias,
        choices: &'a HashMap<Identifier, Vec<Choice>>,
        vars_repository: &'a VarsRepository,
    ) -> Self {
        let execution_sequence = execution_sequence_for_dependencies(vars_repository, alias)
            // TODO fixme
            .map(|e| e.identifiers())
            .map_err(|err| ErrorsUIV2::InitError(Box::new(err)))
            .expect("only valid aliases should be used here");
        let full_name = alias.full_name();

        AliasElement {
            alias,
            choices,
            execution_sequence,
            full_name,
        }
    }
}

#[derive(Clone, Debug)]
struct ChoiceElement<'a> {
    alias: &'a Alias,
    choices: &'a HashMap<Identifier, Vec<Choice>>,
    execution_sequence: &'a [Identifier],
    choice: Choice,
    text: String,
}

impl<'a> ChoiceElement<'a> {
    pub fn from(
        choice: Choice,
        alias: &'a Alias,
        choices: &'a HashMap<Identifier, Vec<Choice>>,
        execution_sequence: &'a [Identifier],
    ) -> Self {
        let text = format!("{}\t{}", choice.value(), choice.desc().unwrap_or_default());
        ChoiceElement {
            alias,
            choices,
            execution_sequence,
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
            "Name:\t{}\n\nDescription:\n{}\n\nAlias:\n\n{}\n",
            self.alias.name(),
            self.alias.desc(),
            self.alias.command(),
        ));

        if !self.execution_sequence.is_empty() {
            output.push_str("\nDependencies:\n");
            for id in self.execution_sequence {
                output.push_str(&format!("- {}\n", id));
            }
        }

        if !self.choices.is_empty() {
            output.push_str("\nCurrent Choices:\n");
            for (id, choice) in self.choices.iter() {
                output.push_str(&format!("- {}\t= {:?}", id, choice));
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
