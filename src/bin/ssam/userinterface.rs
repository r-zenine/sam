use prettytable::{cell, format, row, Table};
use skim::prelude::*;
use ssam::core::aliases::Alias;
use ssam::core::choices::Choice;
use ssam::core::dependencies::{Dependencies, ErrorsResolver, Resolver};
use ssam::core::identifiers::Identifier;
use ssam::io::readers::read_choices;
use ssam::utils::fsutils::{ErrorsFS, TempFile};
use ssam::utils::processes::ShellCommand;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::process::Command;
use termion;
use thiserror::Error;

type UISelector = Arc<dyn SkimItem>;
pub struct UserInterface {
    preview_file: TempFile,
    preview_command: String,
    chosen_alias: Option<Alias>,
    choices: RefCell<HashMap<Identifier, Choice>>,
}

impl UserInterface {
    pub fn new() -> Result<UserInterface, ErrorsUI> {
        let preview_file = TempFile::new()?;
        let preview_command = format!("cat {}", &preview_file.path.as_path().display());
        Ok(UserInterface {
            preview_file,
            preview_command,
            chosen_alias: None,
            choices: RefCell::new(HashMap::new()),
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
        aliases: &Vec<Alias>,
    ) -> Result<AliasItem, ErrorsUI> {
        let choices = aliases.iter().map(AliasItem::from).map(AliasItem::into);
        let idx = self.choose(choices.collect(), prompt)?;
        let selected_alias = aliases
            .get(idx)
            .map(AliasItem::from)
            .ok_or(ErrorsUI::SkimNoSelection)?;
        self.chosen_alias = Some(selected_alias.clone().alias);
        logs::alias(&selected_alias.alias);
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
        Cow::Borrowed(self.alias().name())
    }
}

impl Into<Command> for AliasItem {
    fn into(self) -> Command {
        ShellCommand::as_command(self.alias)
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
    fn resolve_dynamic<CMD>(&self, var: Identifier, cmd: CMD) -> Result<Choice, ErrorsResolver>
    where
        CMD: Into<ShellCommand<String>>,
    {
        let sh_cmd = cmd.into();
        logs::command(&var, &sh_cmd.value());

        let mut to_run = ShellCommand::as_command(sh_cmd);
        let output = to_run
            .output()
            .map_err(|_e| ErrorsResolver::NoChoiceWasAvailable(var.clone()))?;
        let choices = read_choices(output.stdout.as_slice());
        match choices {
            Err(_err) => Err(ErrorsResolver::NoChoiceWasAvailable(var)),
            Ok(v) => self.resolve_static(var, v.into_iter()),
        }
    }

    fn resolve_static(
        &self,
        var: ssam::core::identifiers::Identifier,
        cmd: impl Iterator<Item = Choice>,
    ) -> Result<Choice, ErrorsResolver> {
        let mut choices: Vec<Choice> = cmd.collect();
        if choices.is_empty() {
            return Err(ErrorsResolver::NoChoiceWasAvailable(var.clone()));
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
        logs::choice(&var, &choice);
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
    use ssam::core::aliases::Alias;
    use std::fmt::Display;
    pub fn command(var: impl Display, cmd: impl AsRef<str>) {
        println!(
            "{}{}[SAM][ var = '{}' ]{} Running: '{}'",
            termion::color::Fg(termion::color::Green),
            termion::style::Bold,
            var,
            termion::style::Reset,
            cmd.as_ref(),
        );
    }
    pub fn choice(var: impl Display, choice: impl Display) {
        println!(
            "{}{}[SAM][ var = '{}' ]{} Choice was: '{}'",
            termion::color::Fg(termion::color::Green),
            termion::style::Bold,
            var,
            termion::style::Reset,
            choice,
        );
    }
    pub fn alias(alias: &Alias) {
        println!(
            "{}{}[SAM][ alias = '{}::{}' ]{}",
            termion::color::Fg(termion::color::Green),
            termion::style::Bold,
            alias.namespace().unwrap_or_default(),
            alias.name(),
            termion::style::Reset,
        );
    }
}
