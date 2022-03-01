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

type TerminalBackend = TermionBackend<AlternateScreen<RawTerminal<Stdout>>>;
type TerminalHandle = Terminal<TerminalBackend>;

pub struct UIModal<V: Value> {
    terminal: RefCell<TerminalHandle>,
    theme: UITheme,
    _marker: PhantomData<V>,
}

impl<V: Value> UIModal<V> {
    pub fn new() -> std::io::Result<Self> {
        let stdout = std::io::stdout().into_raw_mode()?;
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(UIModal {
            terminal: RefCell::new(terminal),
            theme: UITheme::default(),
            _marker: PhantomData::default(),
        })
    }
}

impl<V: Value> UIModal<V> {
    pub(super) fn draw(&self, state: &ViewState<V>) {
        let mut terminal = self.terminal.borrow_mut();

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
