use std::fmt;
use std::path::PathBuf;

use ratatui::style::Color;
use serde::de::{self, SeqAccess, Visitor};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub text: ColorValue,
    pub text_dim: ColorValue,
    pub text_muted: ColorValue,
    pub text_inverted: ColorValue,
    pub border_focused: ColorValue,
    pub border_unfocused: ColorValue,
    pub accent: ColorValue,
    pub shadow_fg: ColorValue,
    pub shadow_bg: ColorValue,
    pub blue: ColorValue,
    pub gray: ColorValue,
    pub green: ColorValue,
    pub orange: ColorValue,
    pub pink: ColorValue,
    pub purple: ColorValue,
    pub red: ColorValue,
    pub yellow: ColorValue,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            text: ColorValue(Color::White),
            text_dim: ColorValue(Color::Gray),
            text_muted: ColorValue(Color::DarkGray),
            text_inverted: ColorValue(Color::Black),
            border_focused: ColorValue(Color::Cyan),
            border_unfocused: ColorValue(Color::DarkGray),
            accent: ColorValue(Color::Cyan),
            shadow_fg: ColorValue(Color::Rgb(60, 60, 60)),
            shadow_bg: ColorValue(Color::Rgb(30, 30, 30)),
            blue: ColorValue(Color::Rgb(56, 132, 244)),
            gray: ColorValue(Color::Rgb(155, 163, 176)),
            green: ColorValue(Color::Rgb(75, 210, 143)),
            orange: ColorValue(Color::Rgb(255, 172, 51)),
            pink: ColorValue(Color::Rgb(245, 120, 180)),
            purple: ColorValue(Color::Rgb(163, 113, 247)),
            red: ColorValue(Color::Rgb(244, 81, 81)),
            yellow: ColorValue(Color::Rgb(255, 214, 51)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorValue(pub Color);

impl<'de> Deserialize<'de> for ColorValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(ColorValueVisitor)
    }
}

struct ColorValueVisitor;

impl<'de> Visitor<'de> for ColorValueVisitor {
    type Value = ColorValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a color name (\"cyan\"), hex (\"#3884F4\"), or RGB array ([56, 132, 244])")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<ColorValue, E> {
        parse_color_str(value)
            .map(ColorValue)
            .ok_or_else(|| de::Error::custom(format!("unknown color: \"{value}\"")))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<ColorValue, A::Error> {
        let r: u8 = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &"3 elements for RGB"))?;
        let g: u8 = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &"3 elements for RGB"))?;
        let b: u8 = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(2, &"3 elements for RGB"))?;
        Ok(ColorValue(Color::Rgb(r, g, b)))
    }
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

fn parse_color_str(s: &str) -> Option<Color> {
    if s.starts_with('#') || s.chars().all(|c| c.is_ascii_hexdigit()) && s.len() == 6 {
        return parse_hex_color(s);
    }
    match s.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "dark_gray" | "dark_grey" | "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "light_red" | "lightred" => Some(Color::LightRed),
        "light_green" | "lightgreen" => Some(Color::LightGreen),
        "light_yellow" | "lightyellow" => Some(Color::LightYellow),
        "light_blue" | "lightblue" => Some(Color::LightBlue),
        "light_magenta" | "lightmagenta" => Some(Color::LightMagenta),
        "light_cyan" | "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        "reset" => Some(Color::Reset),
        _ => None,
    }
}

pub fn config_path() -> PathBuf {
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            PathBuf::from(home).join(".config")
        });
    config_dir.join("gh-board").join("config.toml")
}

pub fn load_config() -> anyhow::Result<Config> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {e}", path.display()))?;
    let config: Config = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {e}", path.display()))?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_toml() {
        let config: Config = toml::from_str("").unwrap();
        assert!(matches!(config.theme.text.0, Color::White));
        assert!(matches!(config.theme.accent.0, Color::Cyan));
    }

    #[test]
    fn test_parse_partial_theme() {
        let toml = r#"
[theme]
accent = "red"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(matches!(config.theme.accent.0, Color::Red));
        // unchanged defaults
        assert!(matches!(config.theme.text.0, Color::White));
    }

    #[test]
    fn test_color_value_named() {
        let toml = r#"
[theme]
text = "cyan"
text_dim = "dark_gray"
text_muted = "lightblue"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(matches!(config.theme.text.0, Color::Cyan));
        assert!(matches!(config.theme.text_dim.0, Color::DarkGray));
        assert!(matches!(config.theme.text_muted.0, Color::LightBlue));
    }

    #[test]
    fn test_color_value_hex() {
        let toml = r##"
[theme]
accent = "#FF6600"
"##;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(matches!(config.theme.accent.0, Color::Rgb(255, 102, 0)));
    }

    #[test]
    fn test_color_value_rgb_array() {
        let toml = r#"
[theme]
blue = [100, 149, 237]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(matches!(config.theme.blue.0, Color::Rgb(100, 149, 237)));
    }

    #[test]
    fn test_color_value_invalid() {
        let toml = r#"
[theme]
accent = "not_a_color"
"#;
        let result: Result<Config, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_color_value_rgb_array_wrong_length() {
        let toml = r#"
[theme]
blue = [100, 149]
"#;
        let result: Result<Config, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_full_theme_config() {
        let toml = r##"
[theme]
text = "white"
text_dim = "gray"
text_muted = "dark_gray"
text_inverted = "black"
border_focused = "#00FFFF"
border_unfocused = "dark_gray"
accent = "cyan"
shadow_fg = [60, 60, 60]
shadow_bg = [30, 30, 30]
blue = "#3884F4"
gray = "#9BA3B0"
green = "#4BD28F"
orange = "#FFAC33"
pink = "#F578B4"
purple = "#A371F7"
red = "#F45151"
yellow = "#FFD633"
"##;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(matches!(config.theme.text.0, Color::White));
        assert!(matches!(
            config.theme.border_focused.0,
            Color::Rgb(0, 255, 255)
        ));
        assert!(matches!(
            config.theme.shadow_fg.0,
            Color::Rgb(60, 60, 60)
        ));
    }

    #[test]
    fn test_config_path_xdg() {
        let original = std::env::var("XDG_CONFIG_HOME").ok();
        // SAFETY: test runs single-threaded (--test-threads=1 or isolated)
        unsafe { std::env::set_var("XDG_CONFIG_HOME", "/tmp/test-config") };
        let path = config_path();
        assert_eq!(path, PathBuf::from("/tmp/test-config/gh-board/config.toml"));
        // restore
        match original {
            Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
        }
    }
}
