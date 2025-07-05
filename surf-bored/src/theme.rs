use ratatui::style::{Color, Style, Stylize};
/// Represent colours in theme used by app
#[derive(Clone)]
pub struct Theme {
    name: String,
    text_fg: Color,
    text_bg: Color,
    dimmed_text_fg: Color,
    header_bg: Color,
    hyperlink_style: Style,
}

impl Theme {
    pub fn surf_bored_synth_wave() -> Theme {
        Theme {
            name: "Surf bored synth wave".to_string(),
            text_fg: Color::Rgb(205, 152, 211),
            text_bg: Color::Rgb(23, 21, 41),
            dimmed_text_fg: Color::Rgb(205, 152, 211),
            header_bg: Color::Rgb(109, 228, 175), // bright green header_bg: Color::Rgb(149, 232, 196), // pale green
            hyperlink_style: Style::new().underlined(),
        }
    }

    /// to use for tests so should not be amended
    pub fn default() -> Theme {
        let style = Style::default();
        Theme {
            name: "Default".to_string(),
            text_fg: style.fg.unwrap_or_default(),
            text_bg: style.bg.unwrap_or_default(),
            dimmed_text_fg: style.fg.unwrap_or_default(),
            header_bg: style.bg.unwrap_or_default(),
            hyperlink_style: Style::new().underlined(),
        }
    }

    pub fn header_style(&self) -> Style {
        Style::new().fg(self.text_bg).bg(self.header_bg)
    }

    pub fn text_style(&self) -> Style {
        Style::new().fg(self.text_fg).bg(self.text_bg)
    }

    pub fn inverted_text_style(&self) -> Style {
        Style::new().fg(self.text_bg).bg(self.text_fg)
    }

    pub fn dimmed_text_style(&self) -> Style {
        Style::new().fg(self.dimmed_text_fg).bg(self.text_bg)
    }

    pub fn hyperlink_style(&self) -> Style {
        self.hyperlink_style
    }
}
