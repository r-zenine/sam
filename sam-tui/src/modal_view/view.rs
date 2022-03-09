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
    has_options: bool,
    allow_multi_select: bool,
}

impl<V: Value> Drop for ModalView<V> {
    fn drop(&mut self) {
        self.ui.suspend_raw_mode();
    }
}

impl<V: Value> ModalView<V> {
    pub fn new(list: Vec<V>, options: Vec<OptionToggle>, allow_multi_select: bool) -> Self {
        let has_options = !options.is_empty();
        let state = ViewState::<V>::new(list, options);
        let ui = UIModal::<V>::new().expect("Can't initialize the ui");
        let events = std::io::stdin().keys();
        ModalView {
            state,
            events,
            ui,
            has_options,
            allow_multi_select,
        }
    }

    pub fn run(mut self) -> Option<ViewResponse<V>> {
        self.ui.draw(&self.state);
        if let Some(event) = self.next_event() {
            if event == Event::AppClosed {
                self.ui.suspend_raw_mode();
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
            .map(|ev| ev.and_then(|evt| self.key_transformer(evt)))
            .expect("Can't read")
    }

    fn key_transformer(&self, key: Key) -> Option<Event> {
        match key {
            Key::Backspace | Key::Delete => Some(Event::Backspace),
            Key::Esc if self.has_options => Some(Event::ToggleViewMode),

            Key::Up => Some(Event::Up),
            Key::Down => Some(Event::Down),

            Key::Ctrl('p') => Some(Event::Up),
            Key::Ctrl('n') => Some(Event::Down),

            Key::Ctrl('c') => Some(Event::AppClosed),

            Key::Ctrl('s') if self.allow_multi_select => Some(Event::Mark),
            Key::Ctrl('a') if self.allow_multi_select => Some(Event::MarkAll),

            Key::Char('\n') => Some(Event::Entr),
            Key::Char(c) => Some(Event::InputChar(c)),
            Key::Ctrl(_)
            | Key::Left
            | Key::Right
            | Key::Home
            | Key::End
            | Key::Esc
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
