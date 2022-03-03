use super::list_state::ListState;
use crate::modal_view::state::options_state::OptionToggle;
use crate::modal_view::state::options_state::OptionsState;
use crate::modal_view::state::Event;
use crate::modal_view::state::Value;
use std::collections::HashSet;

#[derive(PartialEq, Debug)]
pub enum ViewMode {
    OptionsMode,
    InsertMode,
}

impl ViewMode {
    fn toggle(&self) -> Self {
        if self == &Self::InsertMode {
            Self::OptionsMode
        } else {
            Self::InsertMode
        }
    }
}

impl Default for ViewMode {
    fn default() -> Self {
        Self::InsertMode
    }
}

#[derive(Debug, Default)]
pub struct ViewState<V: Value> {
    pub current_mod: ViewMode,
    pub list: ListState<V>,
    pub options: OptionsState,
}

#[derive(PartialEq, Debug)]
pub enum ExecutionState {
    Keep,
    ExitSuccess,
    Cancelled,
}

impl<V: Value> ViewState<V> {
    pub fn preview(&self) -> Option<String> {
        self.list
            .highlighted_line
            .and_then(|idx| self.list.current_displayed_values.get(idx))
            .map(|v| v.preview())
    }
}

impl<V: Value> ViewState<V> {
    pub fn new(list: Vec<V>, options: Vec<OptionToggle>) -> Self {
        ViewState::<V> {
            current_mod: ViewMode::default(),
            list: ListState::new(list),
            options: OptionsState::new(options),
        }
    }

    pub fn search_filter(&self) -> &str {
        return self.list.search_filter();
    }

    pub fn update(&mut self, msg: &Event) -> ExecutionState {
        match *msg {
            Event::AppClosed => ExecutionState::Cancelled,
            Event::ToggleViewMode => {
                self.current_mod = self.current_mod.toggle();
                ExecutionState::Keep
            }
            Event::InputChar(c) if self.current_mod == ViewMode::InsertMode => {
                self.list.update_filter(c);
                ExecutionState::Keep
            }
            Event::InputChar(c) if self.current_mod == ViewMode::OptionsMode => {
                self.options.toggle_option(c);
                ExecutionState::Keep
            }

            Event::InputChar(_) => ExecutionState::Keep,
            Event::Backspace if self.current_mod == ViewMode::InsertMode => {
                self.list.remove_last_char_from_filter();
                ExecutionState::Keep
            }
            Event::Up if self.current_mod == ViewMode::InsertMode => {
                self.list.up();
                ExecutionState::Keep
            }
            Event::Down if self.current_mod == ViewMode::InsertMode => {
                self.list.down();
                ExecutionState::Keep
            }
            Event::Mark if self.current_mod == ViewMode::InsertMode => {
                self.list.mark();
                ExecutionState::Keep
            }
            Event::Entr => {
                self.list.entr();
                ExecutionState::ExitSuccess
            }
            Event::MarkAll if self.current_mod == ViewMode::InsertMode => {
                self.list.mark_all();
                ExecutionState::Keep
            }
            _ => ExecutionState::Keep,
        }
    }

    pub fn response(self) -> ViewResponse<V> {
        ViewResponse {
            marked_values: self.list.marked_values(),
            selected_options: self.options.active().collect(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ViewResponse<V: Value> {
    pub marked_values: HashSet<V>,
    pub selected_options: Vec<OptionToggle>,
}

#[cfg(test)]
mod tests {
    use crate::modal_view::state::mocks::MockValue;
    use crate::modal_view::state::Event;
    use crate::modal_view::state::OptionToggle;

    use super::ViewResponse;
    use super::ViewState;

    struct TestCase<'a> {
        input_sequence: &'a [Event],
        initial_list: Vec<MockValue>,
        initial_options: Vec<OptionToggle>,
        expected_response: ViewResponse<MockValue>,
    }

    fn run_case(t: TestCase) {
        let mut view_state = ViewState::new(t.initial_list, t.initial_options);
        for r in t.input_sequence {
            view_state.update(r);
        }
        let response = view_state.response();
        assert_eq!(response, t.expected_response);
    }

    #[test]
    fn test_select_one_element() {
        let case = TestCase {
            input_sequence: &[Event::Down, Event::Entr],
            initial_list: vec![MockValue::new(1, "elem 1"), MockValue::new(2, "elem 2")],
            initial_options: vec![],
            expected_response: ViewResponse {
                marked_values: vec![MockValue::new(2, "elem 2")].into_iter().collect(),
                selected_options: vec![],
            },
        };
        run_case(case)
    }

    #[test]
    fn test_filter_then_select_one_element() {
        let case = TestCase {
            input_sequence: &[Event::Down, Event::InputChar('1'), Event::Entr],
            initial_list: vec![MockValue::new(1, "elem 1"), MockValue::new(2, "elem 2")],
            initial_options: vec![],
            expected_response: ViewResponse {
                marked_values: vec![MockValue::new(1, "elem 1")].into_iter().collect(),
                selected_options: vec![],
            },
        };
        run_case(case)
    }

    #[test]
    fn test_filter_then_select_many_elements() {
        let case_mark_then_entr = TestCase {
            input_sequence: &[
                Event::Down,
                Event::InputChar('1'),
                Event::Mark,
                Event::Up,
                Event::Entr,
            ],
            initial_list: vec![
                MockValue::new(1, "elem 1"),
                MockValue::new(2, "elem 2"),
                MockValue::new(12, "elem 12"),
            ],
            initial_options: vec![],
            expected_response: ViewResponse {
                marked_values: vec![MockValue::new(1, "elem 1"), MockValue::new(12, "elem 12")]
                    .into_iter()
                    .collect(),
                selected_options: vec![],
            },
        };
        run_case(case_mark_then_entr);
        let case_mark_mark_then_entr = TestCase {
            input_sequence: &[
                Event::Down,
                Event::InputChar('1'),
                Event::Mark,
                Event::Up,
                Event::Mark,
                Event::Entr,
            ],
            initial_list: vec![
                MockValue::new(1, "elem 1"),
                MockValue::new(2, "elem 2"),
                MockValue::new(12, "elem 12"),
            ],
            initial_options: vec![],
            expected_response: ViewResponse {
                marked_values: vec![MockValue::new(1, "elem 1"), MockValue::new(12, "elem 12")]
                    .into_iter()
                    .collect(),
                selected_options: vec![],
            },
        };
        run_case(case_mark_mark_then_entr);
    }

    #[test]
    fn toggle_option_select_elements() {
        let case_toggle_option_filter_select = TestCase {
            input_sequence: &[
                Event::Down,
                Event::InputChar('1'),
                Event::Mark,
                Event::Up,
                Event::Mark,
                Event::ToggleViewMode,
                Event::InputChar('o'),
                Event::Entr,
            ],
            initial_list: vec![
                MockValue::new(1, "elem 1"),
                MockValue::new(2, "elem 2"),
                MockValue::new(12, "elem 12"),
            ],
            initial_options: vec![
                OptionToggle {
                    text: String::from("option"),
                    key: 'o',
                    active: false,
                },
                OptionToggle {
                    text: String::from("not option"),
                    key: 'n',
                    active: false,
                },
            ],
            expected_response: ViewResponse {
                marked_values: vec![MockValue::new(1, "elem 1"), MockValue::new(12, "elem 12")]
                    .into_iter()
                    .collect(),
                selected_options: vec![OptionToggle {
                    text: String::from("option"),
                    key: 'o',
                    active: true,
                }],
            },
        };
        run_case(case_toggle_option_filter_select)
    }
}
