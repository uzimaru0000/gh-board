use ratatui::style::Color;

use crate::model::project::ColumnColor;

pub struct ColorTheme {
    // Text
    pub text: Color,
    pub text_dim: Color,
    pub text_muted: Color,
    pub text_inverted: Color,

    // Borders
    pub border_focused: Color,
    pub border_unfocused: Color,

    // Semantic colors
    pub accent: Color,

    // Shadow
    pub shadow_fg: Color,
    pub shadow_bg: Color,

    // Palette
    pub blue: Color,
    pub gray: Color,
    pub green: Color,
    pub orange: Color,
    pub pink: Color,
    pub purple: Color,
    pub red: Color,
    pub yellow: Color,
}

impl ColorTheme {
    pub fn column_color(&self, color: &ColumnColor) -> Color {
        match color {
            ColumnColor::Blue => self.blue,
            ColumnColor::Gray => self.gray,
            ColumnColor::Green => self.green,
            ColumnColor::Orange => self.orange,
            ColumnColor::Pink => self.pink,
            ColumnColor::Purple => self.purple,
            ColumnColor::Red => self.red,
            ColumnColor::Yellow => self.yellow,
        }
    }
}

pub static THEME: ColorTheme = ColorTheme {
    text: Color::White,
    text_dim: Color::Gray,
    text_muted: Color::DarkGray,
    text_inverted: Color::Black,

    border_focused: Color::Cyan,
    border_unfocused: Color::DarkGray,

    accent: Color::Cyan,

    shadow_fg: Color::Rgb(60, 60, 60),
    shadow_bg: Color::Rgb(30, 30, 30),

    blue: Color::Rgb(56, 132, 244),
    gray: Color::Rgb(155, 163, 176),
    green: Color::Rgb(75, 210, 143),
    orange: Color::Rgb(255, 172, 51),
    pink: Color::Rgb(245, 120, 180),
    purple: Color::Rgb(163, 113, 247),
    red: Color::Rgb(244, 81, 81),
    yellow: Color::Rgb(255, 214, 51),
};
