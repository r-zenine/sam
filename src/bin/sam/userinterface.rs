use prettytable::{cell, format, row, Table};
use sam::core::aliases::Alias;
use sam::core::choices::Choice;
use sam::core::dependencies::{Dependencies, ErrorsResolver, Resolver};
use sam::core::identifiers::Identifier;
use sam::io::readers::read_choices;
use sam::utils::fsutils::{ErrorsFS, TempFile};
use sam::utils::processes::ShellCommand;
use skim::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::process::Command;

use thiserror::Error;

use crate::vars_cache::VarsCache;

type UISelector = Arc<dyn SkimItem>;
pub struct UserInterface {
    preview_file: TempFile,
    preview_command: String,
    chosen_alias: Option<Alias>,
    choices: RefCell<HashMap<Identifier, Choice>>,
    silent: bool,
    variables: HashMap<String, String>,
    cache: Box<dyn VarsCache>,
}

impl UserInterface {
    pub fn new(
        silent: bool,
        variables: HashMap<String, String>,
        cache: Box<dyn VarsCache>,
    ) -> Result<UserInterface, ErrorsUI> {
        let preview_file = TempFile::new()?;
        let preview_command = format!("cat {}", &preview_file.path.as_path().display());
        Ok(UserInterface {
            preview_file,
            preview_command,
            chosen_alias: None,
            choices: RefCell::new(HashMap::new()),
            silent,
            variables,
            cache,
        })
    }

    fn skim_options<'ui>(
        prompt: &'ui str,
        preview_command: Option<&'ui str>,
    ) -> Result<SkimOptions<'ui>, ErrorsUI> {
        SkimOptionsBuilder::default()
            .prompt(Some(prompt))
            .preview(preview_command)
            .preview_window(Some("right:wrap"))
            .tabstop(Some("8"))
            .multi(false)
            .no_hscroll(false)
            .algorithm(FuzzyAlgorithm::SkimV2)
            .build()
            .map_err(ErrorsUI::SkimConfig)
    }
    pub fn select_alias(
        &mut self,
        prompt: &'_ str,
        aliases: &[Alias],
    ) -> Result<AliasItem, ErrorsUI> {
        let choices = aliases.iter().map(AliasItem::from).map(AliasItem::into);
        let idx = self.choose(choices.collect(), prompt)?;
        let selected_alias = aliases
            .get(idx)
            .map(AliasItem::from)
            .ok_or(ErrorsUI::SkimNoSelection)?;
        self.chosen_alias = Some(selected_alias.clone().alias);
        if !self.silent {
            logs::alias(&selected_alias.alias);
        }
        Ok(selected_alias)
    }
    pub fn choose(&self, choices: Vec<UISelector>, prompt: &str) -> Result<usize, ErrorsUI> {
        let (s, r) = bounded(choices.len());
        let source = choices.clone();
        iterator_into_sender(source.into_iter(), s)?;
        self.update_preview()?;
        let options = UserInterface::skim_options(prompt, self.preview_command())?;
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

    fn update_preview(&self) -> Result<(), ErrorsUI> {
        if let Some(alias) = &self.chosen_alias {
            let mut handle = self.preview_file.file.borrow_mut();
            (*handle).set_len(0)?;
            writeln!(
                (*handle),
                "\n{}Namespace:{} {}",
                termion::style::Bold,
                termion::style::Reset,
                alias.namespace().unwrap_or("global")
            )?;
            writeln!(
                (*handle),
                "\n{}Alias:{} {}",
                termion::style::Bold,
                termion::style::Reset,
                alias.name()
            )?;
            writeln!(
                (*handle),
                "\n{}Description:{} {}",
                termion::style::Bold,
                termion::style::Reset,
                alias.desc()
            )?;
            writeln!(
                (*handle),
                "\n{}Initial Command:{} {}{}{}{}",
                termion::style::Bold,
                termion::style::Reset,
                termion::style::Bold,
                termion::color::Fg(termion::color::Cyan),
                alias.alias(),
                termion::style::Reset,
            )?;

            writeln!((*handle))?;
            let hashmap = self.choices.borrow();
            if hashmap.len() > 0 {
                writeln!(
                    (*handle),
                    "{}Current Command:{} {}{}{}{}\n",
                    termion::style::Bold,
                    termion::style::Reset,
                    termion::style::Bold,
                    termion::color::Fg(termion::color::Green),
                    alias.substitute_for_choices_partial(&hashmap),
                    termion::style::Reset,
                )?;
                let mut table = Table::new();
                table.set_format(*format::consts::FORMAT_NO_COLSEP);
                table.set_titles(row!["Variable", "Choice"]);
                for (var, choice) in (*hashmap).clone() {
                    table.add_row(row![&var.name(), choice.value()]);
                }
                table.print::<File>(handle.by_ref())?;
            }
            (*handle).flush()?;
        }
        Ok(())
    }

    fn preview_command(&'_ self) -> Option<&'_ str> {
        if self.chosen_alias.is_some() {
            Some(self.preview_command.as_str())
        } else {
            None
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
pub struct AliasItem {
    alias: Alias,
}

impl AliasItem {
    pub fn alias(&self) -> &Alias {
        &self.alias
    }
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

        if !self.silent {
            logs::command(&var, &sh_cmd.value());
        }
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
            if let Some(0) = output.status.code() {
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
        var: sam::core::identifiers::Identifier,
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
        if !self.silent {
            logs::choice(&var, &choice);
        }
        (*mp).insert(var, choice.clone());
        Ok(choice)
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

mod logs {
    use sam::core::aliases::Alias;
    use std::fmt::Display;
    pub fn command(var: impl Display, cmd: impl AsRef<str>) {
        eprintln!(
            "{}{}[SAM][ var = '{}' ]{} Running: '{}'",
            termion::color::Fg(termion::color::Green),
            termion::style::Bold,
            var,
            termion::style::Reset,
            cmd.as_ref(),
        );
    }
    pub fn choice(var: impl Display, choice: impl Display) {
        eprintln!(
            "{}{}[SAM][ var = '{}' ]{} Choice was: '{}'",
            termion::color::Fg(termion::color::Green),
            termion::style::Bold,
            var,
            termion::style::Reset,
            choice,
        );
    }
    pub fn alias(alias: &Alias) {
        eprintln!(
            "{}{}[SAM][ alias = '{}::{}' ]{}",
            termion::color::Fg(termion::color::Green),
            termion::style::Bold,
            alias.namespace().unwrap_or_default(),
            alias.name(),
            termion::style::Reset,
        );
    }
}
