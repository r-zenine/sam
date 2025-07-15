use std::cell::{Cell, RefCell};
use std::io::Stdout;
use std::marker::PhantomData;
use std::time::SystemTime;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::{AlternateScreen, IntoAlternateScreen};
use tui::backend::TermionBackend;

use tui::Terminal;

use super::state::Value;
use super::state::ViewState;
use super::theme::UITheme;
use super::ui_insert_mode::{ListItems, UIInsertMode};
use super::ui_options_mode::UIOptionsMode;

const MIN_TIME_TO_REFRESH_IN_MS: u128 = 75;

pub struct UIModal<V: Value> {
    terminal: RefCell<Terminal<TermionBackend<AlternateScreen<RawTerminal<Stdout>>>>>,
    last_update: Cell<Option<SystemTime>>,

    theme: UITheme,
    _marker: PhantomData<V>,
}

impl<V: Value> UIModal<V> {
    pub fn new() -> std::io::Result<Self> {
        let raw_stdout = std::io::stdout().into_raw_mode()?;
        let screen = raw_stdout.into_alternate_screen()?;
        let backend = TermionBackend::new(screen);
        let terminal = Terminal::new(backend).expect("can't setup terminal");
        
        Ok(UIModal {
            terminal: RefCell::new(terminal),
            last_update: Cell::new(None),
            theme: UITheme::default(),
            _marker: PhantomData::default(),
        })
    }

    pub fn suspend_raw_mode(&mut self) {
        // Note: With the new structure, we can't easily suspend raw mode
        // because the raw terminal is wrapped inside AlternateScreen and TermionBackend
        // You may need to restructure if suspend/resume is critical
        eprintln!();
        println!();
    }
}

impl<V: Value> UIModal<V> {
    pub(super) fn draw(&self, state: &ViewState<V>) {
        if self.enough_time_since_last_refresh() {
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

    fn enough_time_since_last_refresh(&self) -> bool {
        let now = SystemTime::now();
        if let Some(last_time) = self.last_update.get() {
            if last_time.elapsed().expect("can't access clock").as_millis()
                >= MIN_TIME_TO_REFRESH_IN_MS
            {
                self.last_update.replace(Some(now));
                true
            } else {
                false
            }
        } else {
            self.last_update.replace(Some(now));
            true
        }
    }
}
