use std::cell::RefCell;
use std::io::Stdout;
use std::marker::PhantomData;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;

use tui::Terminal;

use super::state::Value;
use super::state::ViewState;
use super::theme::UITheme;
use super::ui_insert_mode::{ListItems, UIInsertMode};
use super::ui_options_mode::UIOptionsMode;

pub struct UIModal<V: Value> {
    raw_terminal: RefCell<RawTerminal<Stdout>>,

    theme: UITheme,
    _marker: PhantomData<V>,
}

impl<V: Value> UIModal<V> {
    pub fn new() -> std::io::Result<Self> {
        let raw_stdout = std::io::stdout().into_raw_mode()?;
        Ok(UIModal {
            raw_terminal: RefCell::new(raw_stdout),
            theme: UITheme::default(),
            _marker: PhantomData::default(),
        })
    }

    pub fn suspend_raw_mode(&mut self) {
        let raw_terminal = &mut *self.raw_terminal.borrow_mut();
        raw_terminal
            .suspend_raw_mode()
            .expect("Can't suspect raw mode");
    }
}

impl<V: Value> UIModal<V> {
    pub(super) fn draw(&self, state: &ViewState<V>) {
        let raw_terminal = &mut *self.raw_terminal.borrow_mut();
        let stdout = AlternateScreen::from(raw_terminal);
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend).expect("can't setup terminal");

        terminal
            .draw(|f| {
                match state.current_mod {
                    super::state::ViewMode::OptionsMode => {
                        let options_mode_view = UIOptionsMode::new(&self.theme);
                        options_mode_view.draw(f, &state.options)
                    }
                    super::state::ViewMode::InsertMode => {
                        let insert_mode_view = UIInsertMode::new(f.size(), &self.theme);
                        insert_mode_view.draw(
                            f,
                            ListItems::from(state),
                            state.search_filter(),
                            state.preview().unwrap_or_default().as_str(),
                        )
                    }
                };
            })
            .expect("Can't draw");
    }
}
