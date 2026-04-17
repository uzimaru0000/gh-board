use ratatui::style::Color;

pub fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_hash_prefix() {
        assert_eq!(parse_hex_color("#3884F4"), Some(Color::Rgb(0x38, 0x84, 0xF4)));
    }

    #[test]
    fn parses_without_hash_prefix() {
        assert_eq!(parse_hex_color("3884F4"), Some(Color::Rgb(0x38, 0x84, 0xF4)));
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(parse_hex_color("#FFF"), None);
        assert_eq!(parse_hex_color("#FFFFFFF"), None);
    }

    #[test]
    fn rejects_non_hex() {
        assert_eq!(parse_hex_color("#ZZZZZZ"), None);
    }
}
