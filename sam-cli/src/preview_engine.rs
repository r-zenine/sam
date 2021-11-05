use std::{collections::HashMap, io::Write};
use thiserror::Error;

use sam_core::{
    aliases::Alias,
    choices::Choice,
    commands::Command,
    dependencies::ErrorsResolver,
    identifiers::Identifier,
    repositories::{
        AliasesRepository, ErrorsAliasesRepository, ErrorsVarsRepository, VarsRepository,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum PreviewCommand {
    PreviewAlias { alias_id: Identifier },
}

pub struct PreviewEngine {
    pub aliases: AliasesRepository,
    pub vars: VarsRepository,
    pub defaults: HashMap<Identifier, Choice>,
    pub output: Box<dyn Write>,
}

impl PreviewEngine {
    pub fn run(&mut self, command: PreviewCommand) -> Result<i32> {
        use PreviewCommand::*;
        match command {
            PreviewAlias { alias_id } => self.preview_alias(alias_id),
        }
    }

    fn preview_alias(&mut self, alias_id: Identifier) -> Result<i32> {
        let choices: &HashMap<Identifier, Choice> = &self.defaults;
        let alias: Alias = self.aliases.get(&alias_id)?.with_partial_choices(&choices);
        let exec_seq = self.vars.execution_sequence(alias.clone())?;

        write!(
            self.output,
            "{}Name:{}\t{}\n\n",
            termion::style::Bold,
            termion::style::Reset,
            alias_id,
        )?;
        write!(
            self.output,
            "{}Description:{}\n{}\n\n",
            termion::style::Bold,
            termion::style::Reset,
            alias.desc()
        )?;
        write!(
            self.output,
            "{}Alias:{}\n\n{}\n",
            termion::style::Bold,
            termion::style::Reset,
            alias.command(),
        )?;

        if !exec_seq.identifiers().is_empty() {
            write!(
                self.output,
                "\n{}Dependencies:{}\n",
                termion::style::Bold,
                termion::style::Reset,
            )?;
            for id in exec_seq.identifiers() {
                write!(self.output, "- {}\n", id)?;
            }
        }

        if !choices.is_empty() {
            write!(
                self.output,
                "\n{}Current Choices:{}\n",
                termion::style::Bold,
                termion::style::Reset,
            )?;
            for (id, choice) in choices.iter() {
                write!(self.output, "- {}\t= {}\n", id, choice)?;
            }
        }

        Ok(0)
    }
}

type Result<T> = std::result::Result<T, ErrorsPreviewEngine>;

#[derive(Debug, Error)]
pub enum ErrorsPreviewEngine {
    #[error("Can't write to output\n-> {0}")]
    ErrorOutput(#[from] std::io::Error),
    #[error("Can't retrieve requested alias\n-> {0}")]
    ErrorAliasesRepository(#[from] ErrorsAliasesRepository),
    #[error("Can't figure out execution sequence\n-> {0}")]
    ErrorVarsRepository(#[from] ErrorsVarsRepository),
    #[error("Can't substitute provided choices\n-> {0}")]
    ErrorsChoiceSubstituion(#[from] ErrorsResolver),
}
