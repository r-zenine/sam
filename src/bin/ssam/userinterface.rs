use crossbeam_channel;
use skim::prelude::*;
use ssam::core::aliases::Alias;
use ssam::core::vars::{Choice, ErrorsVarResolver, VarName, VarResolver};
use ssam::io::readers::read_choices;
use ssam::utils::processes::ShellCommand;
use std::process::Command;

// Todo :
// 1. use a Cell for options to allow for mutations.
// 2. add a reset options to change the prompt from the resolver.
#[derive(Debug, Default)]
pub struct UserInterface {}

impl UserInterface {
    fn skim_options(prompt: &'_ str) -> Result<SkimOptions, ErrorsUI> {
        SkimOptionsBuilder::default()
            .prompt(Some(prompt))
            .tabstop(Some("8"))
            .multi(false)
            .no_hscroll(false)
            .algorithm(FuzzyAlgorithm::SkimV2)
            .build()
            .map_err(|op| ErrorsUI::SkimConfig(op))
    }

    pub fn select_alias(
        &self,
        prompt: &'_ str,
        aliases: &Vec<Alias>,
    ) -> Result<AliasItem, ErrorsUI> {
        let choices = aliases.clone().into_iter().map(AliasItem::arc_alias);
        let idx = self.choose(choices.clone().collect(), prompt)?;
        aliases
            .get(idx)
            .map(|e| AliasItem::from_alias(e.to_owned()))
            .ok_or(ErrorsUI::SkimNoSelection)
    }

    pub fn choose(&self, choices: Vec<Arc<dyn SkimItem>>, prompt: &str) -> Result<usize, ErrorsUI> {
        let (s, r) = bounded(choices.len());
        let source = choices.clone();
        iterator_into_sender(source.into_iter(), s)?;
        let options = UserInterface::skim_options(prompt)?;
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
}
#[derive(Debug)]
pub enum ErrorsUI {
    SkimConfig(String),
    SkimSend(String),
    SkimNoSelection,
    SkimAborted,
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
        self.choose(items, prompt.as_str())
            .map_err(|_e| ErrorsVarResolver::NoChoiceWasSelected(var.clone()))
            .and_then(|idx| {
                choices
                    .get(idx)
                    .map(|e| e.to_owned())
                    .ok_or(ErrorsVarResolver::NoChoiceWasSelected(var.clone()))
            })
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
