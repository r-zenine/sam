use crate::preview_skim::PreviewSkim;
use sam_core::entities::aliases::Alias;
use sam_core::entities::choices::Choice;
use sam_core::entities::dependencies::{ErrorsResolver, Resolver};
use sam_core::entities::identifiers::Identifier;
use sam_core::entities::processes::ShellCommand;
use sam_readers::read_choices;
use sam_utils::fsutils::ErrorsFS;
use skim::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::process::Command;

use thiserror::Error;

use sam_persistence::VarsCache;

type UISelector = Arc<dyn SkimItem>;

pub struct UserInterface {
    selected_identifier: RefCell<Option<Identifier>>,
    choices: RefCell<HashMap<Identifier, Choice>>,
    variables: HashMap<String, String>,
    cache: Box<dyn VarsCache>,
}

impl UserInterface {
    pub fn new(
        variables: HashMap<String, String>,
        cache: Box<dyn VarsCache>,
    ) -> Result<UserInterface, ErrorsUI> {
        Ok(UserInterface {
            selected_identifier: RefCell::default(),
            choices: RefCell::new(HashMap::new()),
            variables,
            cache,
        })
    }
    pub fn with_identifier(
        selected_identifier: Identifier,
        variables: HashMap<String, String>,
        cache: Box<dyn VarsCache>,
    ) -> Result<UserInterface, ErrorsUI> {
        Ok(UserInterface {
            selected_identifier: RefCell::new(Some(selected_identifier)),
            choices: RefCell::new(HashMap::new()),
            variables,
            cache,
        })
    }

    fn skim_options<'ui>(
        prompt: &'ui str,
        preview_command: &'ui str,
    ) -> Result<SkimOptions<'ui>, ErrorsUI> {
        SkimOptionsBuilder::default()
            .prompt(Some(prompt))
            .preview(Some(preview_command))
            .preview_window(Some("right:wrap"))
            .tabstop(Some("8"))
            .multi(false)
            .no_hscroll(false)
            .algorithm(FuzzyAlgorithm::SkimV2)
            .build()
            .map_err(ErrorsUI::SkimConfig)
    }

    pub fn choose(&self, choices: Vec<UISelector>, prompt: &str) -> Result<usize, ErrorsUI> {
        let (s, r) = bounded(choices.len());
        let source = choices.clone();
        iterator_into_sender(source.into_iter(), s)?;
        let preview_command = self.preview_command();
        let options = UserInterface::skim_options(prompt, &preview_command)?;
        let output = Skim::run_with(&options, Some(r)).ok_or(ErrorsUI::SkimNoSelection)?;

        if output.is_abort {
            return Err(ErrorsUI::SkimAborted);
        }

        let selection = output
            .selected_items
            .get(0)
            .ok_or(ErrorsUI::SkimNoSelection)?;

        let item = choices
            .iter()
            .enumerate()
            .find(|(_idx, value)| value.text() == selection.text());

        match item {
            Some((idx, _)) => Ok(idx),
            None => Err(ErrorsUI::SkimNoSelection),
        }
    }

    fn preview_command(&'_ self) -> String {
        let borrowed_choices = self.choices.borrow();
        let preview = PreviewSkim::new(&borrowed_choices);
        if let Some(alias) = self.selected_identifier.borrow().deref() {
            preview.preview_for_identifier(alias)
        } else {
            preview.preview()
        }
    }
}
#[derive(Debug, Error)]
pub enum ErrorsUI {
    #[error("could not configure the user interface because\n-> {0}")]
    SkimConfig(String),
    #[error("could not initialize the user interface because\n-> {0}")]
    SkimSend(String),
    #[error("no selection was provided")]
    SkimNoSelection,
    #[error("the program was aborted")]
    SkimAborted,
    #[error("an unexpected error happend while filling the preview window {0}")]
    IOError(#[from] std::io::Error),
    #[error("an unexpected error happend while initialising the preview window {0}")]
    FSError(#[from] ErrorsFS),
}

#[derive(Clone, Debug)]
pub struct IdentifierWithDescItem {
    identifier: Identifier,
    description: Option<String>,
}

impl From<Identifier> for IdentifierWithDescItem {
    fn from(identifier: Identifier) -> Self {
        IdentifierWithDescItem {
            identifier,
            description: None,
        }
    }
}

impl From<&Identifier> for IdentifierWithDescItem {
    fn from(identifier: &Identifier) -> Self {
        IdentifierWithDescItem {
            identifier: identifier.clone(),
            description: None,
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<UISelector> for IdentifierWithDescItem {
    fn into(self) -> UISelector {
        Arc::new(self)
    }
}

impl SkimItem for IdentifierWithDescItem {
    fn text(&self) -> Cow<str> {
        Cow::Owned(format!("{}", self.identifier,))
    }
}

#[derive(Clone, Debug)]
pub struct AliasItem {
    alias: Alias,
}

impl From<Alias> for AliasItem {
    fn from(alias: Alias) -> Self {
        AliasItem { alias }
    }
}

impl From<&Alias> for AliasItem {
    fn from(alias: &Alias) -> Self {
        AliasItem {
            alias: alias.to_owned(),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<UISelector> for AliasItem {
    fn into(self) -> UISelector {
        Arc::new(self)
    }
}

impl SkimItem for AliasItem {
    fn text(&self) -> Cow<str> {
        Cow::Owned(format!("{}\t{}", self.alias.full_name(), self.alias.desc()))
    }
}

#[allow(clippy::from_over_into)]
impl Into<Command> for AliasItem {
    fn into(self) -> Command {
        ShellCommand::make_command(self.alias)
    }
}

struct ChoiceItem {
    inner: Choice,
}

impl ChoiceItem {
    fn from_choice(choice: Choice) -> UISelector {
        Arc::new(ChoiceItem { inner: choice })
    }
}

impl SkimItem for ChoiceItem {
    fn text(&self) -> Cow<str> {
        Cow::Owned(format!(
            "{}\t{}",
            self.inner.value(),
            self.inner.desc().unwrap_or(""),
        ))
    }
}

impl Resolver for UserInterface {
    fn resolve_input(&self, var: Identifier, prompt: &str) -> Result<Choice, ErrorsResolver> {
        let mut buffer = String::new();
        println!(
            "Please provide an input for variable {}.\n{} :",
            &var, prompt
        );
        match std::io::stdin().read_line(&mut buffer) {
            Ok(_) => Ok(Choice::new(buffer.replace("\n", ""), None)),
            Err(err) => Err(ErrorsResolver::NoInputWasProvided(var, err.to_string())),
        }
    }
    fn resolve_dynamic<CMD>(&self, var: Identifier, cmd: CMD) -> Result<Choice, ErrorsResolver>
    where
        CMD: Into<ShellCommand<String>>,
    {
        let sh_cmd = cmd.into();
        let cmd_key = sh_cmd
            .replace_env_vars_in_command(&self.variables)
            .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.clone(), Box::new(e)))?;

        let cache_entry = self.cache.get(cmd_key.value());
        let (stdout_output, stderr) = if let Ok(Some(out)) = cache_entry {
            (out.as_bytes().to_owned(), vec![])
        } else {
            let mut to_run = ShellCommand::make_command(sh_cmd.clone());
            to_run.envs(&self.variables);
            let output = to_run
                .output()
                .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.clone(), e.into()))?;
            if output.status.code() == Some(0) && output.stderr.is_empty() {
                self.cache
                    .put(
                        cmd_key.value(),
                        &String::from_utf8_lossy(output.stdout.as_slice()).to_owned(),
                    )
                    .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.clone(), Box::new(e)))?;
            }
            (output.stdout, output.stderr)
        };

        let choices = read_choices(stdout_output.as_slice());
        match choices {
            Err(e) => Err(ErrorsResolver::DynamicResolveFailure(var, e.into())),
            Ok(v) if !v.is_empty() => self.resolve_static(var, v.into_iter()),
            Ok(_) => Err(ErrorsResolver::DynamicResolveEmpty(
                var,
                sh_cmd.value().to_owned(),
                std::str::from_utf8(&stderr).unwrap_or("").to_owned(),
            )),
        }
    }

    fn resolve_static(
        &self,
        var: Identifier,
        cmd: impl Iterator<Item = Choice>,
    ) -> Result<Choice, ErrorsResolver> {
        let mut choices: Vec<Choice> = cmd.collect();
        if choices.is_empty() {
            return Err(ErrorsResolver::NoChoiceWasAvailable(var));
        }
        if choices.len() == 1 {
            return Ok(choices.pop().unwrap());
        }
        let items: Vec<UISelector> = choices
            .clone()
            .into_iter()
            .map(ChoiceItem::from_choice)
            .collect();
        let prompt = format!("please make a choices for variable:\t{}", var.name());
        let choice = self
            .choose(items, prompt.as_str())
            .map_err(|_e| ErrorsResolver::NoChoiceWasSelected(var.clone()))
            .and_then(|idx| {
                choices
                    .get(idx)
                    .map(|e| e.to_owned())
                    .ok_or_else(|| ErrorsResolver::NoChoiceWasSelected(var.clone()))
            })?;
        let mut mp = self.choices.borrow_mut();
        (*mp).insert(var, choice.clone());
        Ok(choice)
    }

    fn select_identifier(
        &self,
        identifiers: &[Identifier],
        descriptions: Option<&[&str]>,
        prompt: &str,
    ) -> Result<Identifier, ErrorsResolver> {
        let items: Vec<UISelector> = identifiers
            .iter()
            .enumerate()
            .map(|(i, identifier)| {
                IdentifierWithDescItem {
                    identifier: identifier.clone(),
                    // TODO handle descriptions
                    description: descriptions
                        .and_then(|descs| descs.get(i))
                        .map(ToString::to_string),
                }
                .into()
            })
            .collect();
        let idx = self
            .choose(items, prompt)
            .map_err(|e| ErrorsResolver::IdentifierSelectionInvalid(Box::new(e)))?;
        let identifier = identifiers
            .get(idx)
            .cloned()
            .ok_or(ErrorsResolver::IdentifierSelectionEmpty())?;
        self.selected_identifier.replace(Some(identifier.clone()));
        Ok(identifier)
    }
}

fn iterator_into_sender<I, U>(it: I, s: Sender<U>) -> Result<(), ErrorsUI>
where
    U: Clone,
    I: Iterator<Item = U>,
{
    it.fold(Ok(()), |acc, e| {
        acc.and_then(|_| s.send(e).map_err(|op| ErrorsUI::SkimSend(op.to_string())))
    })?;
    Ok(())
}
