use std::sync::OnceLock;

use ratatui::style::Color;

use crate::config::ThemeConfig;
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

static THEME: OnceLock<ColorTheme> = OnceLock::new();

pub fn init_theme(config: &ThemeConfig) {
    let theme = ColorTheme {
        text: config.text.0,
        text_dim: config.text_dim.0,
        text_muted: config.text_muted.0,
        text_inverted: config.text_inverted.0,
        border_focused: config.border_focused.0,
        border_unfocused: config.border_unfocused.0,
        accent: config.accent.0,
        shadow_fg: config.shadow_fg.0,
        shadow_bg: config.shadow_bg.0,
        blue: config.blue.0,
        gray: config.gray.0,
        green: config.green.0,
        orange: config.orange.0,
        pink: config.pink.0,
        purple: config.purple.0,
        red: config.red.0,
        yellow: config.yellow.0,
    };
    let _ = THEME.set(theme);
}

pub fn theme() -> &'static ColorTheme {
    THEME.get_or_init(|| {
        let default = ThemeConfig::default();
        ColorTheme {
            text: default.text.0,
            text_dim: default.text_dim.0,
            text_muted: default.text_muted.0,
            text_inverted: default.text_inverted.0,
            border_focused: default.border_focused.0,
            border_unfocused: default.border_unfocused.0,
            accent: default.accent.0,
            shadow_fg: default.shadow_fg.0,
            shadow_bg: default.shadow_bg.0,
            blue: default.blue.0,
            gray: default.gray.0,
            green: default.green.0,
            orange: default.orange.0,
            pink: default.pink.0,
            purple: default.purple.0,
            red: default.red.0,
            yellow: default.yellow.0,
        }
    })
}
