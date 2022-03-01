use tui::style::{Color, Modifier, Style};

pub struct UITheme {
    pub background: Color,
    pub foreground: Color,
    pub highlight: Color,
    pub borders: Color,
}

impl UITheme {
    pub(super) fn style(&self) -> Style {
        Style::default().fg(self.foreground).bg(self.background)
    }
    pub(super) fn highlight_style(&self) -> Style {
        Style::default()
            .add_modifier(Modifier::ITALIC)
            .bg(self.foreground)
            .fg(self.highlight)
    }
}

impl Default for UITheme {
    fn default() -> Self {
        Self {
            foreground: Color::Rgb(209, 208, 208),
            background: Color::Rgb(38, 38, 38),
            highlight: Color::Rgb(87, 85, 127),
            borders: Color::Rgb(104, 134, 209),
        }
    }
}
