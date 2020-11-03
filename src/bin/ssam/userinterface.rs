use crossbeam_channel;
use skim::prelude::*;
use ssam::core::aliases::Alias;
use ssam::core::scripts::Script;
use ssam::core::vars::{Choice, ErrorsVarResolver, VarName, VarResolver};
use ssam::io::readers::read_choices;
use ssam::utils::processes::ShellCommand;
use std::process::Command;

// Todo :
// 1. use a Cell for options to allow for mutations.
// 2. add a reset options to change the prompt from the resolver.
pub struct UserInterface<'a> {
    options: SkimOptions<'a>,
}

impl<'a> UserInterface<'a> {
    pub fn new(prompt: &'a str) -> Result<UserInterface<'a>, UIError> {
        let skimoptions = SkimOptionsBuilder::default()
            .prompt(Some(prompt))
            .tabstop(Some("8"))
            .multi(false)
            .no_hscroll(false)
            .algorithm(FuzzyAlgorithm::SkimV2)
            .build()
            .map_err(|op| UIError::ErrorSkimConfig(op))?;

        Ok(UserInterface {
            options: skimoptions,
        })
    }

    pub fn run(&self, aliases: Vec<Alias>, scripts: Vec<Script>) -> Result<UIItem, UIError> {
        let aliases_strings = aliases.clone().into_iter().map(UIItem::make_alias);
        let scripts_strings = scripts.clone().into_iter().map(UIItem::make_script);
        let choices = aliases_strings.chain(scripts_strings);
        let idx = self.choose(choices.clone().collect())?;
        if idx >= aliases.len() {
            scripts
                .get(idx - aliases.len())
                .map(|e| UIItem::from_script(e.to_owned()))
                .ok_or(UIError::ErrorSkimNoSelection)
        } else {
            aliases
                .get(idx)
                .map(|e| UIItem::from_alias(e.to_owned()))
                .ok_or(UIError::ErrorSkimNoSelection)
        }
    }

    pub fn choose(&self, choices: Vec<Arc<dyn SkimItem>>) -> Result<usize, UIError> {
        let (s, r) = bounded(choices.len());
        let source = choices.clone();
        UserInterface::send_from_iterator(source.into_iter(), s)?;
        let output = Skim::run_with(&self.options, Some(r)).ok_or(UIError::ErrorSkimNoSelection)?;
        if output.is_abort {
            return Err(UIError::ErrorSkimAborted);
        }
        let selection: &dyn SkimItem = output.selected_items[0].as_ref();

        let item = choices
            .iter()
            .enumerate()
            .filter(|(_idx, value)| value.text() == selection.text())
            .next();

        match item {
            Some((idx, _)) => Ok(idx),
            None => Err(UIError::ErrorSkimNoSelection),
        }
    }
    fn send_from_iterator<I>(it: I, s: Sender<Arc<dyn SkimItem>>) -> Result<(), UIError>
    where
        I: Iterator<Item = Arc<dyn SkimItem>>,
    {
        it.fold(Ok(()), |acc, e| {
            acc.and_then(|_| {
                s.send(e)
                    .map_err(|op| UIError::ErrorSkimSend(op.clone().to_string()))
            })
        })?;
        Ok(())
    }
}
#[derive(Debug)]
pub enum UIError {
    ErrorSkimConfig(String),
    ErrorSkimSend(String),
    ErrorSkimNoSelection,
    ErrorSkimAborted,
}

impl From<crossbeam_channel::SendError<Arc<dyn skim::SkimItem>>> for UIError {
    fn from(_: crossbeam_channel::SendError<Arc<dyn skim::SkimItem>>) -> Self {
        todo!()
    }
}
#[derive(Clone, Debug)]
pub enum UIItemKind {
    Alias,
    Script,
}

#[derive(Clone, Debug)]
pub struct UIItem {
    pub kind: UIItemKind,
    alias: Option<Alias>,
    script: Option<Script>,
}

impl UIItem {
    fn from_alias(alias: Alias) -> UIItem {
        UIItem {
            kind: UIItemKind::Alias,
            alias: Some(alias),
            script: None,
        }
    }
    fn make_alias(alias: Alias) -> Arc<dyn SkimItem> {
        Arc::new(Self::from_alias(alias))
    }

    fn from_script(script: Script) -> UIItem {
        UIItem {
            kind: UIItemKind::Script,
            alias: None,
            script: Some(script),
        }
    }
    fn make_script(script: Script) -> Arc<dyn SkimItem> {
        Arc::new(Self::from_script(script))
    }

    pub fn as_alias(&self) -> Option<&Alias> {
        match self.kind {
            UIItemKind::Alias => self.alias.as_ref(),
            UIItemKind::Script => None,
        }
    }

    pub fn as_script(&self) -> Option<&Script> {
        match self.kind {
            UIItemKind::Alias => None,
            UIItemKind::Script => self.script.as_ref(),
        }
    }
}

impl SkimItem for UIItem {
    fn text(&self) -> Cow<str> {
        match &self.kind {
            UIItemKind::Alias => Cow::Owned(
                self.alias
                    .as_ref()
                    .map(|e| e.into())
                    .unwrap_or("".to_string()),
            ),
            UIItemKind::Script => Cow::Owned(
                self.script
                    .as_ref()
                    .map(|e| e.into())
                    .unwrap_or("".to_string()),
            ),
        }
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
            "{} \t {}",
            self.inner.value(),
            self.inner.desc().unwrap_or(""),
        ))
    }
}

impl<'a> VarResolver for UserInterface<'a> {
    fn resolve_dynamic<CMD>(&self, var: VarName, cmd: CMD) -> Result<Choice, ErrorsVarResolver>
    where
        CMD: Into<ShellCommand<String>>,
    {
        let mut to_run = ShellCommand::as_command(cmd.into());
        let output = to_run
            .output()
            .map_err(|_e| ErrorsVarResolver::ErrorNoChoiceWasAvailable(var.clone()))?;
        let choices = read_choices(output.stdout.as_slice());
        match choices {
            Err(_err) => Err(ErrorsVarResolver::ErrorNoChoiceWasAvailable(var.clone())),
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
        self.choose(items)
            .map_err(|_e| ErrorsVarResolver::ErrorNoChoiceWasSelected(var.clone()))
            .and_then(|idx| {
                choices
                    .get(idx)
                    .map(|e| e.to_owned())
                    .ok_or(ErrorsVarResolver::ErrorNoChoiceWasSelected(var.clone()))
            })
    }
}

impl Into<Command> for UIItem {
    fn into(self) -> Command {
        match &self.kind {
            UIItemKind::Alias => ShellCommand::as_command(self.alias.unwrap()),
            UIItemKind::Script => ShellCommand::as_command(self.script.unwrap()),
        }
    }
}
