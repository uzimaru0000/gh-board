use std::collections::VecDeque;

use super::project::Board;

/// サーバーサイドフィルタのクエリ組み合わせごとに Board をキャッシュする LRU。
/// 同じフィルタを再適用したときに server 応答を待たず即時表示するのが目的。
pub struct BoardCache {
    entries: VecDeque<(Vec<String>, Board)>,
    max: usize,
}

impl BoardCache {
    pub fn new(max: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max,
        }
    }

    /// 指定 key に対応する Board を clone して返す。ヒット時は LRU 的に先頭へ昇格。
    pub fn get(&mut self, key: &[String]) -> Option<Board> {
        let pos = self.entries.iter().position(|(k, _)| k.as_slice() == key)?;
        let entry = self.entries.remove(pos).unwrap();
        let board = entry.1.clone();
        self.entries.push_front(entry);
        Some(board)
    }

    /// 既存の同 key エントリを置き換えた上で先頭に挿入。容量超過時は末尾を除去。
    pub fn put(&mut self, key: Vec<String>, board: Board) {
        self.entries.retain(|(k, _)| k != &key);
        self.entries.push_front((key, board));
        while self.entries.len() > self.max {
            self.entries.pop_back();
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub fn keys(&self) -> Vec<Vec<String>> {
        self.entries.iter().map(|(k, _)| k.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board_with_title(title: &str) -> Board {
        Board {
            project_title: title.to_string(),
            status_field_id: String::new(),
            columns: vec![],
            repositories: vec![],
        }
    }

    #[test]
    fn test_miss_returns_none() {
        let mut cache = BoardCache::new(4);
        assert!(cache.get(&["label:\"bug\"".to_string()]).is_none());
    }

    #[test]
    fn test_put_then_get() {
        let mut cache = BoardCache::new(4);
        let key = vec!["label:\"bug\"".to_string()];
        cache.put(key.clone(), board_with_title("bug board"));
        let got = cache.get(&key).unwrap();
        assert_eq!(got.project_title, "bug board");
    }

    #[test]
    fn test_put_replaces_existing_key() {
        let mut cache = BoardCache::new(4);
        let key = vec!["label:\"bug\"".to_string()];
        cache.put(key.clone(), board_with_title("v1"));
        cache.put(key.clone(), board_with_title("v2"));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&key).unwrap().project_title, "v2");
    }

    #[test]
    fn test_lru_evicts_oldest() {
        let mut cache = BoardCache::new(2);
        cache.put(vec!["a".into()], board_with_title("a"));
        cache.put(vec!["b".into()], board_with_title("b"));
        cache.put(vec!["c".into()], board_with_title("c"));
        // oldest "a" is evicted
        assert!(cache.get(&["a".to_string()]).is_none());
        assert!(cache.get(&["b".to_string()]).is_some());
        assert!(cache.get(&["c".to_string()]).is_some());
    }

    #[test]
    fn test_get_promotes_to_front() {
        let mut cache = BoardCache::new(2);
        cache.put(vec!["a".into()], board_with_title("a"));
        cache.put(vec!["b".into()], board_with_title("b"));
        // touch "a" -> now "b" is oldest
        cache.get(&["a".to_string()]);
        cache.put(vec!["c".into()], board_with_title("c"));
        assert!(cache.get(&["a".to_string()]).is_some());
        assert!(cache.get(&["b".to_string()]).is_none()); // evicted
        assert!(cache.get(&["c".to_string()]).is_some());
    }

    #[test]
    fn test_empty_key_works_as_unfiltered_slot() {
        let mut cache = BoardCache::new(4);
        cache.put(vec![], board_with_title("all"));
        assert_eq!(cache.get(&[]).unwrap().project_title, "all");
    }

    #[test]
    fn test_clear_removes_all() {
        let mut cache = BoardCache::new(4);
        cache.put(vec!["a".into()], board_with_title("a"));
        cache.put(vec!["b".into()], board_with_title("b"));
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.get(&["a".to_string()]).is_none());
    }
}
