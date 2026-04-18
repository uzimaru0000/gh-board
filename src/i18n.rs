pub fn init() {
    let locale = detect_locale(
        std::env::var("LC_ALL").ok().as_deref(),
        std::env::var("LC_MESSAGES").ok().as_deref(),
        std::env::var("LANG").ok().as_deref(),
    );
    rust_i18n::set_locale(locale);
}

fn detect_locale(lc_all: Option<&str>, lc_messages: Option<&str>, lang: Option<&str>) -> &'static str {
    let raw = [lc_all, lc_messages, lang]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|v| !v.is_empty());

    match raw.map(primary_language) {
        Some("ja") => "ja",
        _ => "en",
    }
}

fn primary_language(locale: &str) -> &str {
    locale
        .split(['.', '@'])
        .next()
        .unwrap_or("")
        .split(['_', '-'])
        .next()
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ja_from_lang() {
        assert_eq!(detect_locale(None, None, Some("ja_JP.UTF-8")), "ja");
        assert_eq!(detect_locale(None, None, Some("ja")), "ja");
        assert_eq!(detect_locale(None, None, Some("ja-JP")), "ja");
    }

    #[test]
    fn defaults_to_en_for_other_locales() {
        assert_eq!(detect_locale(None, None, Some("en_US.UTF-8")), "en");
        assert_eq!(detect_locale(None, None, Some("C")), "en");
        assert_eq!(detect_locale(None, None, Some("fr_FR")), "en");
    }

    #[test]
    fn defaults_to_en_for_missing_and_empty() {
        assert_eq!(detect_locale(None, None, None), "en");
        assert_eq!(detect_locale(Some(""), Some(""), Some("")), "en");
    }

    #[test]
    fn lc_all_overrides_others() {
        assert_eq!(
            detect_locale(Some("ja_JP.UTF-8"), Some("en_US.UTF-8"), Some("en_US.UTF-8")),
            "ja"
        );
        assert_eq!(
            detect_locale(Some("en_US.UTF-8"), Some("ja_JP.UTF-8"), Some("ja_JP.UTF-8")),
            "en"
        );
    }

    #[test]
    fn lc_messages_overrides_lang() {
        assert_eq!(
            detect_locale(None, Some("ja_JP.UTF-8"), Some("en_US.UTF-8")),
            "ja"
        );
    }

    #[test]
    fn handles_modifier_suffix() {
        assert_eq!(detect_locale(None, None, Some("ja_JP.UTF-8@currency=JPY")), "ja");
    }
}
