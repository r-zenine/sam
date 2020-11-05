use prettytable::{cell, format, row, Table};
use skim::prelude::*;
use ssam::core::aliases::Alias;
use ssam::core::vars::{Choice, ErrorsVarResolver, VarName, VarResolver};
use ssam::io::readers::read_choices;
use ssam::utils::fsutils::{ErrorsFS, TempFile};
use ssam::utils::processes::ShellCommand;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::process::Command;

// Todo :
// 1. use a Cell for options to allow for mutations.
// 2. add a reset options to change the prompt from the resolver.
pub struct UserInterface {
    preview_file: TempFile,
    chosen_alias: Option<Alias>,
    preview_command: String,
    choices: RefCell<HashMap<VarName, Choice>>,
}

impl UserInterface {
    pub fn new() -> Result<UserInterface, ErrorsUI> {
        let preview_file = TempFile::new()?;
        let preview_command = format!("cat {}", &preview_file.path.as_path().display());
        Ok(UserInterface {
            preview_file: preview_file,
            preview_command: preview_command,
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
            .tabstop(Some("8"))
            .multi(false)
            .no_hscroll(false)
            .algorithm(FuzzyAlgorithm::SkimV2)
            .build()
            .map_err(|op| ErrorsUI::SkimConfig(op))
    }

    pub fn select_alias(
        &mut self,
        prompt: &'_ str,
        aliases: &Vec<Alias>,
    ) -> Result<AliasItem, ErrorsUI> {
        let choices = aliases.clone().into_iter().map(AliasItem::arc_alias);
        let idx = self.choose(choices.clone().collect(), prompt)?;
        let selected_alias = aliases
            .get(idx)
            .ok_or(ErrorsUI::SkimNoSelection)
            .map(|e| AliasItem::from_alias(e.to_owned()))?;
        self.chosen_alias = Some(selected_alias.clone().alias);
        Ok(selected_alias)
    }

    pub fn choose(&self, choices: Vec<Arc<dyn SkimItem>>, prompt: &str) -> Result<usize, ErrorsUI> {
        let (s, r) = bounded(choices.len());
        let source = choices.clone();
        iterator_into_sender(source.into_iter(), s)?;
        self.update_preview()?;
        let options = UserInterface::skim_options(prompt, self.preview_command())?;
        let output = Skim::run_with(&options, Some(r)).ok_or(ErrorsUI::SkimNoSelection)?;
        if output.is_abort {
            return Err(ErrorsUI::SkimAborted);
        }
        let selection: &dyn SkimItem = output.selected_items[0].as_ref();
        let item = choices
            .iter()
            .enumerate()
            .filter(|(_idx, value)| value.text() == selection.text())
            .next();

        match item {
            Some((idx, _)) => Ok(idx),
            None => Err(ErrorsUI::SkimNoSelection),
        }
    }

    fn update_preview(&self) -> Result<(), ErrorsUI> {
        if let Some(alias) = &self.chosen_alias {
            let mut handle = self.preview_file.file.borrow_mut();
            (*handle).set_len(0)?;
            // (*handle).seek(SeekFrom::Start(0))?;
            writeln!((*handle), "Alias: {}", alias.name())?;
            writeln!((*handle), "Description: {}", alias.desc())?;
            writeln!((*handle), "Command: {}", alias.alias())?;
            writeln!((*handle), "")?;
            let hashmap = self.choices.borrow();
            if hashmap.len() > 0 {
                let mut table = Table::new();
                table.set_format(*format::consts::FORMAT_NO_COLSEP);
                table.set_titles(row!["Variable", "Choice"]);
                for (var, choice) in (*hashmap).clone() {
                    table.add_row(row![&var, choice.value()]);
                }
                table.print::<File>(handle.by_ref())?;
            }
            (*handle).flush()?;
        }
        Ok(())
    }

    fn preview_command<'ui>(&'ui self) -> Option<&'ui str> {
        if let Some(_) = self.chosen_alias {
            Some(self.preview_command.as_str())
        } else {
            None
        }
    }
}
#[derive(Debug)]
pub enum ErrorsUI {
    SkimConfig(String),
    SkimSend(String),
    SkimNoSelection,
    SkimAborted,
    IOError(std::io::Error),
    FSError(ErrorsFS),
}

impl From<ErrorsFS> for ErrorsUI {
    fn from(v: ErrorsFS) -> Self {
        ErrorsUI::FSError(v)
    }
}

impl From<std::io::Error> for ErrorsUI {
    fn from(v: std::io::Error) -> Self {
        ErrorsUI::IOError(v)
    }
}

#[derive(Clone, Debug)]
pub struct AliasItem {
    alias: Alias,
}

impl AliasItem {
    fn from_alias(alias: Alias) -> AliasItem {
        AliasItem { alias }
    }
    fn arc_alias(alias: Alias) -> Arc<dyn SkimItem> {
        Arc::new(Self::from_alias(alias))
    }

    pub fn alias(&self) -> &Alias {
        &self.alias
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
    fn from_choice(choice: Choice) -> Arc<dyn SkimItem> {
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

impl VarResolver for UserInterface {
    fn resolve_dynamic<CMD>(&self, var: VarName, cmd: CMD) -> Result<Choice, ErrorsVarResolver>
    where
        CMD: Into<ShellCommand<String>>,
    {
        let mut to_run = ShellCommand::as_command(cmd.into());
        let output = to_run
            .output()
            .map_err(|_e| ErrorsVarResolver::NoChoiceWasAvailable(var.clone()))?;
        let choices = read_choices(output.stdout.as_slice());
        match choices {
            Err(_err) => Err(ErrorsVarResolver::NoChoiceWasAvailable(var.clone())),
            Ok(v) => self.resolve_static(var, v.into_iter()),
        }
    }

    fn resolve_static(
        &self,
        var: ssam::core::vars::VarName,
        cmd: impl Iterator<Item = Choice>,
    ) -> Result<Choice, ErrorsVarResolver> {
        let choices: Vec<Choice> = cmd.collect();
        let items: Vec<Arc<dyn SkimItem>> = choices
            .clone()
            .into_iter()
            .map(ChoiceItem::from_choice)
            .collect();
        let prompt = format!("please make a choices for variable:\t{}", var.as_ref());
        let choice = self
            .choose(items, prompt.as_str())
            .map_err(|_e| ErrorsVarResolver::NoChoiceWasSelected(var.clone()))
            .and_then(|idx| {
                choices
                    .get(idx)
                    .map(|e| e.to_owned())
                    .ok_or(ErrorsVarResolver::NoChoiceWasSelected(var.clone()))
            })?;
        let mut mp = self.choices.borrow_mut();
        (*mp).insert(var.clone(), choice.clone());
        Ok(choice)
    }
}

fn iterator_into_sender<I, U>(it: I, s: Sender<U>) -> Result<(), ErrorsUI>
where
    U: Clone,
    I: Iterator<Item = U>,
{
    it.fold(Ok(()), |acc, e| {
        acc.and_then(|_| {
            s.send(e)
                .map_err(|op| ErrorsUI::SkimSend(op.clone().to_string()))
        })
    })?;
    Ok(())
}
