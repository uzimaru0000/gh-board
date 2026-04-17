//! GitHub Releases API を使った最新リリースの確認。
//!
//! TUI 起動時にバックグラウンドで最新 release の tag を取得し、現在の
//! `CARGO_PKG_VERSION` より新しければ `AppEvent::UpdateAvailable` を発火する。
//! 認証不要 (public API) で、失敗はすべてサイレント無視。

const RELEASES_URL: &str =
    "https://api.github.com/repos/uzimaru0000/gh-board/releases/latest";

/// 最新 release の tag を取得し、`v` プレフィックスを除去した version 文字列で返す。
/// ネットワークエラーや JSON 解析失敗などは `None` を返す。
pub async fn fetch_latest_version() -> Option<String> {
    let resp = reqwest::Client::new()
        .get(RELEASES_URL)
        .header("User-Agent", "gh-board")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    let tag = json.get("tag_name")?.as_str()?;
    Some(strip_v_prefix(tag).to_string())
}

fn strip_v_prefix(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

/// `latest` が `current` より新しいバージョンかを判定する。
/// どちらかが `major.minor.patch` 形式として解析できない場合は `false`。
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_semver(latest), parse_semver(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let core = s.split(['-', '+']).next().unwrap_or(s);
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((parts[0].parse().ok()?, parts[1].parse().ok()?, parts[2].parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_v_prefix() {
        assert_eq!(strip_v_prefix("v1.2.3"), "1.2.3");
        assert_eq!(strip_v_prefix("1.2.3"), "1.2.3");
        assert_eq!(strip_v_prefix("version-1"), "ersion-1");
    }

    #[test]
    fn is_newer_detects_higher_patch() {
        assert!(is_newer("1.0.1", "1.0.0"));
    }

    #[test]
    fn is_newer_detects_higher_minor() {
        assert!(is_newer("1.2.0", "1.1.99"));
    }

    #[test]
    fn is_newer_detects_higher_major() {
        assert!(is_newer("2.0.0", "1.99.99"));
    }

    #[test]
    fn is_newer_false_on_equal() {
        assert!(!is_newer("1.0.0", "1.0.0"));
    }

    #[test]
    fn is_newer_false_on_older() {
        assert!(!is_newer("0.9.0", "1.0.0"));
    }

    #[test]
    fn is_newer_false_on_unparseable() {
        assert!(!is_newer("abc", "1.0.0"));
        assert!(!is_newer("1.0.0", "abc"));
    }

    #[test]
    fn parse_semver_ignores_prerelease_suffix() {
        assert_eq!(parse_semver("1.2.3-rc.1"), Some((1, 2, 3)));
        assert_eq!(parse_semver("1.2.3+build.5"), Some((1, 2, 3)));
    }
}
