use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::model::project::{Board, ProjectSummary};

const CACHE_VERSION: u32 = 1;
const DEFAULT_TTL_SECS: u64 = 24 * 60 * 60;

/// 起動高速化のためのディスクキャッシュ。`(owner, project_number, group_by)` 単位で
/// `ProjectSummary` と `Board` をペアで保存する。次回起動時にキャッシュがあれば
/// API レスポンスを待たずに即時表示し、バックグラウンドで stale-while-revalidate する。
pub struct DiskCache {
    root: Option<PathBuf>,
    ttl: Duration,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    version: u32,
    saved_at: u64,
    project: ProjectSummary,
    board: Board,
}

#[derive(Debug, Clone)]
pub struct CachedBoard {
    pub project: ProjectSummary,
    pub board: Board,
}

impl DiskCache {
    /// `~/.cache/gh-board/board/` (XDG 準拠) を root にしたインスタンスを返す。
    /// cache dir 解決に失敗した場合は no-op キャッシュとして振る舞う。
    pub fn new() -> Self {
        let root = dirs::cache_dir().map(|p| p.join("gh-board").join("board"));
        Self {
            root,
            ttl: Duration::from_secs(DEFAULT_TTL_SECS),
        }
    }

    /// 完全に無効化されたインスタンス (`--no-cache` 用)。get/put が即座に no-op になる。
    pub fn disabled() -> Self {
        Self {
            root: None,
            ttl: Duration::from_secs(0),
        }
    }

    fn entry_path(&self, key: &CacheKey) -> Option<PathBuf> {
        let root = self.root.as_ref()?;
        Some(root.join(format!("{}.json", key.file_stem())))
    }

    /// キャッシュから取得。期限切れ・バージョン不一致・パース失敗は None を返し
    /// 必要であれば壊れたファイルを削除する。
    pub fn get(&self, key: &CacheKey) -> Option<CachedBoard> {
        let path = self.entry_path(key)?;
        let bytes = fs::read(&path).ok()?;
        let entry: CacheEntry = match serde_json::from_slice(&bytes) {
            Ok(e) => e,
            Err(_) => {
                let _ = fs::remove_file(&path);
                return None;
            }
        };
        if entry.version != CACHE_VERSION {
            let _ = fs::remove_file(&path);
            return None;
        }
        if !self.is_fresh(entry.saved_at) {
            let _ = fs::remove_file(&path);
            return None;
        }
        Some(CachedBoard {
            project: entry.project,
            board: entry.board,
        })
    }

    /// キャッシュへ保存。書き込みエラーは無視 (キャッシュは best-effort)。
    pub fn put(&self, key: &CacheKey, project: &ProjectSummary, board: &Board) {
        let Some(path) = self.entry_path(key) else {
            return;
        };
        if let Some(parent) = path.parent()
            && fs::create_dir_all(parent).is_err()
        {
            return;
        }
        let entry = CacheEntry {
            version: CACHE_VERSION,
            saved_at: now_unix(),
            project: project.clone(),
            board: board.clone(),
        };
        let Ok(bytes) = serde_json::to_vec(&entry) else {
            return;
        };
        let tmp = path.with_extension("json.tmp");
        if fs::write(&tmp, &bytes).is_ok() {
            let _ = fs::rename(&tmp, &path);
        }
    }

    /// 全キャッシュを破棄。mutation 後に呼ぶことで stale データ表示を防ぐ。
    pub fn invalidate_all(&self) {
        let Some(root) = self.root.as_ref() else {
            return;
        };
        let Ok(entries) = fs::read_dir(root) else {
            return;
        };
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let _ = fs::remove_file(path);
            }
        }
    }

    fn is_fresh(&self, saved_at: u64) -> bool {
        let now = now_unix();
        now.saturating_sub(saved_at) < self.ttl.as_secs()
    }

    #[cfg(test)]
    pub fn with_root(root: PathBuf) -> Self {
        Self {
            root: Some(root),
            ttl: Duration::from_secs(DEFAULT_TTL_SECS),
        }
    }

    #[cfg(test)]
    pub fn with_root_and_ttl(root: PathBuf, ttl: Duration) -> Self {
        Self { root: Some(root), ttl }
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// キャッシュキー。owner / project_number / group_by の組み合わせで Board が変わるため
/// すべて含める。`@me` はキャッシュ汚染を避けるため `viewer_login` で正規化される想定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheKey {
    pub owner: String,
    pub number: i32,
    pub group_by: Option<String>,
}

impl CacheKey {
    pub fn new(owner: impl Into<String>, number: i32, group_by: Option<String>) -> Self {
        Self {
            owner: owner.into(),
            number,
            group_by,
        }
    }

    /// ファイル名として安全な形式へ変換。`/`, スペース等を `_` に置換。
    fn file_stem(&self) -> String {
        let owner = sanitize(&self.owner);
        let group = self.group_by.as_deref().map(sanitize).unwrap_or_default();
        if group.is_empty() {
            format!("{}_{}_default", owner, self.number)
        } else {
            format!("{}_{}_{}", owner, self.number, group)
        }
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::project::Grouping;

    fn tmp_root() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("gh-board-cache-test-{}-{}", std::process::id(), now_unix_nanos()));
        p
    }

    fn now_unix_nanos() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    }

    fn sample_project() -> ProjectSummary {
        ProjectSummary {
            id: "PVT_xxx".into(),
            title: "My Project".into(),
            number: 5,
            description: None,
            url: "https://example.com".into(),
        }
    }

    fn sample_board() -> Board {
        Board {
            project_title: "My Project".into(),
            grouping: Grouping::None,
            columns: vec![],
            repositories: vec![],
            field_definitions: vec![],
        }
    }

    #[test]
    fn put_then_get_round_trips() {
        let root = tmp_root();
        let cache = DiskCache::with_root(root.clone());
        let key = CacheKey::new("octocat", 5, None);

        assert!(cache.get(&key).is_none());
        cache.put(&key, &sample_project(), &sample_board());
        let got = cache.get(&key).expect("hit");
        assert_eq!(got.project.id, "PVT_xxx");
        assert_eq!(got.board.project_title, "My Project");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn expired_entries_return_none_and_are_removed() {
        let root = tmp_root();
        let cache = DiskCache::with_root_and_ttl(root.clone(), Duration::from_secs(0));
        let key = CacheKey::new("octocat", 5, None);
        cache.put(&key, &sample_project(), &sample_board());
        // TTL=0 なので即座に期限切れ
        assert!(cache.get(&key).is_none());
        // ファイルが削除されていること
        let path = cache.entry_path(&key).unwrap();
        assert!(!path.exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn invalidate_all_clears_directory() {
        let root = tmp_root();
        let cache = DiskCache::with_root(root.clone());
        let k1 = CacheKey::new("a", 1, None);
        let k2 = CacheKey::new("b", 2, Some("Status".into()));
        cache.put(&k1, &sample_project(), &sample_board());
        cache.put(&k2, &sample_project(), &sample_board());
        cache.invalidate_all();
        assert!(cache.get(&k1).is_none());
        assert!(cache.get(&k2).is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn group_by_creates_distinct_keys() {
        let root = tmp_root();
        let cache = DiskCache::with_root(root.clone());
        let k1 = CacheKey::new("a", 1, None);
        let k2 = CacheKey::new("a", 1, Some("Sprint".into()));
        cache.put(&k1, &sample_project(), &sample_board());
        let mut p2 = sample_project();
        p2.title = "Sprint board".into();
        let mut b2 = sample_board();
        b2.project_title = "Sprint board".into();
        cache.put(&k2, &p2, &b2);
        assert_eq!(cache.get(&k1).unwrap().board.project_title, "My Project");
        assert_eq!(cache.get(&k2).unwrap().board.project_title, "Sprint board");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sanitize_handles_problematic_owner() {
        let root = tmp_root();
        let cache = DiskCache::with_root(root.clone());
        let key = CacheKey::new("my/org name", 7, Some("Group/By".into()));
        cache.put(&key, &sample_project(), &sample_board());
        assert!(cache.get(&key).is_some());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn version_mismatch_is_dropped() {
        let root = tmp_root();
        let cache = DiskCache::with_root(root.clone());
        let key = CacheKey::new("a", 1, None);
        let path = cache.entry_path(&key).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        // 不正なバージョンで直接書き込む
        let bad = serde_json::json!({
            "version": 99,
            "saved_at": now_unix(),
            "project": sample_project(),
            "board": sample_board(),
        });
        fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(cache.get(&key).is_none());
        assert!(!path.exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn disabled_cache_is_noop() {
        let cache = DiskCache::disabled();
        let key = CacheKey::new("a", 1, None);
        cache.put(&key, &sample_project(), &sample_board());
        assert!(cache.get(&key).is_none());
        cache.invalidate_all(); // panic しないこと
    }
}
