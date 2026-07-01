use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    #[default]
    Mist,
    Ink,
    Paper,
}

impl ThemeName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mist => "mist",
            Self::Ink => "ink",
            Self::Paper => "paper",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteTheme {
    pub title_fg: Color,
    pub query_fg: Color,
    pub filter_fg: Color,
    pub muted_fg: Color,
    pub selected_fg: Color,
    pub selected_bg: Color,
}

impl From<ThemeName> for PaletteTheme {
    fn from(value: ThemeName) -> Self {
        match value {
            ThemeName::Mist => Self {
                title_fg: Color::White,
                query_fg: Color::Yellow,
                filter_fg: Color::Cyan,
                muted_fg: Color::DarkGray,
                selected_fg: Color::Black,
                selected_bg: Color::Cyan,
            },
            ThemeName::Ink => Self {
                title_fg: Color::LightCyan,
                query_fg: Color::LightYellow,
                filter_fg: Color::LightBlue,
                muted_fg: Color::DarkGray,
                selected_fg: Color::White,
                selected_bg: Color::Blue,
            },
            ThemeName::Paper => Self {
                title_fg: Color::White,
                query_fg: Color::LightMagenta,
                filter_fg: Color::LightGreen,
                muted_fg: Color::Gray,
                selected_fg: Color::Black,
                selected_bg: Color::Yellow,
            },
        }
    }
}

impl PaletteTheme {
    pub fn title_style(self) -> Style {
        Style::default()
            .fg(self.title_fg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn query_style(self) -> Style {
        Style::default().fg(self.query_fg)
    }

    pub fn filter_style(self) -> Style {
        Style::default().fg(self.filter_fg)
    }

    pub fn muted_style(self) -> Style {
        Style::default().fg(self.muted_fg)
    }

    pub fn selected_style(self) -> Style {
        Style::default()
            .fg(self.selected_fg)
            .bg(self.selected_bg)
            .add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn theme_names_map_to_distinct_palette_colors() {
        assert_eq!(PaletteTheme::from(ThemeName::Mist).selected_bg, Color::Cyan);
        assert_eq!(PaletteTheme::from(ThemeName::Ink).selected_bg, Color::Blue);
        assert_eq!(
            PaletteTheme::from(ThemeName::Paper).selected_bg,
            Color::Yellow
        );
    }
}
