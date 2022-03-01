use std::io::Stdin;

use crate::modal_view::state::Event;
use termion::input::{Keys, TermRead};

use super::{
    state::{OptionToggle, Value, ViewResponse, ViewState},
    ui::UIModal,
};
use termion::event::Key;

pub struct ModalView<V: Value> {
    state: ViewState<V>,
    ui: UIModal<V>,
    events: Keys<Stdin>,
}

impl<V: Value> ModalView<V> {
    pub fn new(list: Vec<V>, options: Vec<OptionToggle>) -> Self {
        let state = ViewState::<V>::new(list, options);
        let ui = UIModal::<V>::new().expect("Can't initialize the ui");
        let events = std::io::stdin().keys();
        ModalView { state, events, ui }
    }

    pub fn run(mut self) -> Option<ViewResponse<V>> {
        self.ui.draw(&self.state);
        if let Some(event) = self.next_event() {
            if event == Event::AppClosed {
                return None;
            }
            let status = self.state.update(&event);
            self.ui.draw(&self.state);
            match status {
                super::state::ExecutionState::Keep => self.run(),
                super::state::ExecutionState::ExitSuccess => Some(self.state.response()),
                super::state::ExecutionState::Cancelled => None,
            }
        } else {
            self.run()
        }
    }
    pub fn next_event(&mut self) -> Option<Event> {
        self.events
            .next()
            .transpose()
            .map(|ev| ev.and_then(Self::key_transformer))
            .expect("Can't read")
    }

    fn key_transformer(key: Key) -> Option<Event> {
        match key {
            Key::Backspace | Key::Delete => Some(Event::Backspace),
            Key::Esc => Some(Event::ToggleViewMode),

            Key::Up => Some(Event::Up),
            Key::Down => Some(Event::Down),

            Key::Ctrl('p') => Some(Event::Up),
            Key::Ctrl('n') => Some(Event::Down),

            Key::Ctrl('c') => Some(Event::AppClosed),

            Key::Ctrl('s') => Some(Event::Mark),
            Key::Ctrl('a') => Some(Event::MarkAll),

            Key::Char('\n') => Some(Event::Entr),
            Key::Char(c) => Some(Event::InputChar(c)),
            Key::Ctrl(_)
            | Key::Left
            | Key::Right
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::BackTab
            | Key::Insert
            | Key::F(_)
            | Key::Alt(_)
            | Key::Null
            | Key::__IsNotComplete => None,
        }
    }
}

#[cfg(test)]
mod test {

    use super::ModalView;
    use crate::modal_view::{state::mocks::MockValue, OptionToggle};

    #[test]
    fn test_controller_run() {
        let initial_list = vec![
            MockValue::new(1, "elem 1"),
            MockValue::new(2, "elem 2"),
            MockValue::new(12, "elem 12"),
        ];
        let initial_options = vec![
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
        ];
        let mut controller = ModalView::new(initial_list, initial_options);
        let response = controller.run();
        panic!("{:?}", response);
    }
}
