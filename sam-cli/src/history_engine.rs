use sam_core::{
    algorithms::{resolver::Resolver, VarsCollection, VarsDefaultValues},
    engines::{
        AliasCollection, ErrorSamEngine, SamCommand::ExecuteAlias, SamEngine, SamHistory,
        VarsDefaultValuesSetter,
    },
    entities::identifiers::Identifier,
};
use sam_persistence::{AliasHistory, ErrorAliasHistory, HistoryEntry};
use sam_tui::modal_view::{ModalView, Value};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub enum HistoryCommand {
    InterractWithHistory,
    ExecuteLastExecutedAlias,
}

pub struct HistoryEngine<
    R: Resolver,
    AR: AliasCollection,
    VR: VarsCollection,
    DV: VarsDefaultValuesSetter + VarsDefaultValues,
> {
    pub sam_engine: SamEngine<R, AR, VR, DV>,
    pub history: AliasHistory,
}

impl<
        R: Resolver,
        AR: AliasCollection,
        VR: VarsCollection,
        DV: VarsDefaultValues + VarsDefaultValuesSetter,
    > HistoryEngine<R, AR, VR, DV>
{
    pub fn run(&mut self, command: HistoryCommand) -> Result<i32> {
        match command {
            HistoryCommand::InterractWithHistory => self.interract_with_history(),
            HistoryCommand::ExecuteLastExecutedAlias => self.execute_last_executed_alias(),
        }
    }

    fn interract_with_history(&mut self) -> Result<i32> {
        let history_entries: Vec<HistoryEntryWrapper> =
            self.history.entries()?.map(HistoryEntryWrapper).collect();
        if !history_entries.is_empty() {
            let controller = ModalView::new(history_entries, vec![], false);
            let response = controller.run();
            let selection_o = response
                .and_then(|v| v.values().take(1).next())
                .map(|e| e.0);
            if let Some(selection) = selection_o {
                let selection_id = selection.r.name();

                self.sam_engine.aliases.get(selection_id).ok_or_else(|| {
                    ErrorHistoryEngine::AliasNotAvailable(selection_id.clone(), selection.pwd)
                })?;

                self.sam_engine.defaults.set_defaults(selection.r.choices());

                self.sam_engine.run(ExecuteAlias {
                    alias: selection.r.name().clone(),
                })?;
            }
        }
        Ok(0)
    }

    fn execute_last_executed_alias(&self) -> Result<i32> {
        let resolved_alias_o = self.history.get_last()?;
        if let Some(alias) = resolved_alias_o {
            Ok(self
                .sam_engine
                .executor
                .execute_resolved_alias(&alias, &self.sam_engine.env_variables)?)
        } else {
            println!("history empty");
            Ok(0)
        }
    }
}

#[derive(Debug, Clone)]
struct HistoryEntryWrapper(HistoryEntry);

impl Eq for HistoryEntryWrapper {}

impl PartialEq for HistoryEntryWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl std::hash::Hash for HistoryEntryWrapper {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.r.commands().hash(state);
    }
}

impl Value for HistoryEntryWrapper {
    fn text(&self) -> &str {
        self.0.r.name().name()
    }

    fn preview(&self) -> String {
        format!("{}", self.0.r)
    }
}

pub type Result<T> = std::result::Result<T, ErrorHistoryEngine>;
#[derive(Debug, Error)]
pub enum ErrorHistoryEngine {
    #[error("could not run a command\n-> {0}")]
    SamEngine(#[from] ErrorSamEngine),
    #[error("alias {0} unavailable, last time it was ran from directory: {1}")]
    AliasNotAvailable(Identifier, String),
    #[error("could not read from history\n-> {0}")]
    History(#[from] ErrorAliasHistory),
}
