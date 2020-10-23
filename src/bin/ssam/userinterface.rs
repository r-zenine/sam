use crossbeam_channel;
use skim::prelude::*;
use ssam::core::aliases::Alias;
use ssam::core::scripts::Script;
use ssam::utils::processes::ShellCommand;
use std::process::Command;

pub struct UserInterface<'a> {
    options: SkimOptions<'a>,
}

const PROMPT: &'_ str = "Choose a script/alias > ";

impl<'a> UserInterface<'a> {
    pub fn new() -> Result<UserInterface<'a>, UIError> {
        let skimoptions = SkimOptionsBuilder::default()
            .prompt(Some(PROMPT))
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
        let (s, r) = bounded(&aliases.len() + &scripts.len());
        let aliases_strings = aliases.clone().into_iter().map(UIItem::make_alias);
        let scripts_strings = scripts.clone().into_iter().map(UIItem::make_script);
        let source = aliases_strings.chain(scripts_strings);
        UserInterface::send_from_iterator(source, s)?;
        let output = Skim::run_with(&self.options, Some(r)).ok_or(UIError::ErrorSkimNoSelection)?;
        let selection: &dyn SkimItem = output.selected_items[0].as_ref();
        let item: &UIItem = selection
            .as_any()
            .downcast_ref::<UIItem>()
            .ok_or(UIError::ErrorSkimDowncast)?;
        match item.kind {
            UIItemKind::Alias => {
                let selected = aliases
                    .into_iter()
                    .find(|e| item.as_alias().as_ref().map(|r| *r == e).unwrap_or(false));
                return selected
                    .map(|a| UIItem::from_alias(a))
                    .ok_or(UIError::ErrorSelection);
            }
            UIItemKind::Script => {
                let selected = scripts
                    .into_iter()
                    .find(|e| item.as_script().as_ref().map(|r| *r == e).unwrap_or(false));
                return selected
                    .map(|s| UIItem::from_script(s))
                    .ok_or(UIError::ErrorSelection);
            }
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
    ErrorSelection,
    ErrorSkimConfig(String),
    ErrorSkimSend(String),
    ErrorSkimNoSelection,
    ErrorSkimDowncast,
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
    kind: UIItemKind,
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

    fn as_alias(&self) -> Option<&Alias> {
        match self.kind {
            UIItemKind::Alias => self.alias.as_ref(),
            UIItemKind::Script => None,
        }
    }

    fn as_script(&self) -> Option<&Script> {
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

impl Into<Command> for UIItem {
    fn into(self) -> Command {
        match &self.kind {
            UIItemKind::Alias => ShellCommand::as_command(self.alias.unwrap()),
            UIItemKind::Script => ShellCommand::as_command(self.script.unwrap()),
        }
    }
}
