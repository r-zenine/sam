mod list_state;
mod options_state;
mod view_state;

pub use view_state::ExecutionState;

pub use options_state::OptionToggle;
pub use options_state::OptionsState;
pub use view_state::ViewMode;
pub use view_state::ViewState;

pub use view_state::ViewResponse;

pub trait Value: Eq + std::hash::Hash + Clone + std::fmt::Debug {
    fn text(&self) -> &str;
    fn preview(&self) -> String;
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    AppClosed,
    ToggleViewMode,
    InputChar(char),
    Backspace,
    Entr,
    Up,
    Down,
    Mark,
    MarkAll,
}

pub mod mocks {
    use super::Value;

    #[derive(Eq, Hash, Clone, Debug, PartialEq, Default)]
    pub struct MockValue(usize, String);

    impl MockValue {
        pub fn new(id: usize, msg: &str) -> Self {
            MockValue(id, msg.to_string())
        }
    }
    impl Value for MockValue {
        fn text(&self) -> &str {
            &self.1
        }

        fn preview(&self) -> String {
            self.1.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::modal_view::state::{view_state::ViewMode, ViewState};

    use super::mocks::MockValue;

    #[test]
    fn test_state_transitions_input_and_normal_mode() {
        let mut state = ViewState::<MockValue>::default();
        // we make sure that the default is input mode
        assert_eq!(state.current_mod, ViewMode::InsertMode);
        state.update(&super::Event::ToggleViewMode);
        assert_eq!(state.current_mod, ViewMode::OptionsMode);
        state.update(&super::Event::ToggleViewMode);
        assert_eq!(state.current_mod, ViewMode::InsertMode);
    }

    #[test]
    fn test_state_transitions_search_filters() {
        let mut state = ViewState::<MockValue>::default();
        state.update(&super::Event::InputChar('c'));
        state.update(&super::Event::InputChar('o'));
        state.update(&super::Event::ToggleViewMode);
        state.update(&super::Event::InputChar('o'));
        state.update(&super::Event::ToggleViewMode);
        state.update(&super::Event::Backspace);
        let filter = state.search_filter();
        assert_eq!(filter, "c");
    }

    #[test]
    fn test_state_transitions_options_toggles() {}

    #[test]
    fn test_state_transitions_options_list_navigation() {}
}
