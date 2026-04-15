//! Roadmap view の時間軸計算 (pure functions)。
//!
//! `roadmap_timeline` は iterations と今日の日付から、表示対象ウィンドウ
//! (今日を含む ±2 iteration) と各セグメントの列幅を決定する。

use crate::model::project::IterationOption;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineSegment {
    pub iteration_id: String,
    pub title: String,
    pub start_date: String,
    pub duration: i32,
    pub completed: bool,
    pub contains_today: bool,
    pub start_col: u16,
    pub width: u16,
}

/// 表示対象 iteration と描画列を計算する。
///
/// * `iterations` — 全 iteration (sorted されていなくても良い)
/// * `today` — 現在日付 (Some(y, m, d)) または None。None なら最初の非完了 iteration を中心にする
/// * `total_width` — 描画領域の幅 (列数)
///
/// 戻り値: 表示対象の iteration 一覧 (start_col, width 付き)。空 iteration / 幅 0 なら空ベクタ。
pub fn roadmap_timeline(
    iterations: &[IterationOption],
    today: Option<(i32, u32, u32)>,
    total_width: u16,
) -> Vec<TimelineSegment> {
    if iterations.is_empty() || total_width == 0 {
        return Vec::new();
    }

    let mut sorted_indices: Vec<usize> = (0..iterations.len()).collect();
    sorted_indices.sort_by(|&a, &b| iterations[a].start_date.cmp(&iterations[b].start_date));

    let starts: Vec<Option<i64>> = iterations
        .iter()
        .map(|it| date_to_days(&it.start_date))
        .collect();
    let today_days = today.map(|(y, m, d)| days_from_civil(y, m, d));

    // 今日を含む iteration → 今日より後の最初の iteration → 末尾の順でセンターを決定
    let center = if let Some(td) = today_days {
        sorted_indices
            .iter()
            .position(|&i| {
                starts[i]
                    .is_some_and(|s| td >= s && td < s + iterations[i].duration.max(1) as i64)
            })
            .or_else(|| {
                sorted_indices
                    .iter()
                    .position(|&i| starts[i].is_some_and(|s| s > td))
            })
            .unwrap_or(sorted_indices.len().saturating_sub(1))
    } else {
        sorted_indices
            .iter()
            .position(|&i| !iterations[i].completed)
            .unwrap_or(sorted_indices.len().saturating_sub(1))
    };

    let win_start = center.saturating_sub(2);
    let win_end = (center + 3).min(sorted_indices.len());
    let window: Vec<usize> = sorted_indices[win_start..win_end].to_vec();

    let total_duration: i64 = window
        .iter()
        .map(|&i| iterations[i].duration.max(1) as i64)
        .sum();
    if total_duration <= 0 {
        return Vec::new();
    }

    let mut segments = Vec::with_capacity(window.len());
    let mut cursor: u16 = 0;
    let last_idx = window.len() - 1;
    for (n, &i) in window.iter().enumerate() {
        let it = &iterations[i];
        let w = if n == last_idx {
            // 末尾セグメントで端数を吸収
            total_width.saturating_sub(cursor)
        } else {
            let raw = (it.duration.max(1) as i64) * total_width as i64 / total_duration;
            raw as u16
        };
        let contains_today = today_days.is_some_and(|td| {
            starts[i].is_some_and(|s| td >= s && td < s + it.duration.max(1) as i64)
        });
        segments.push(TimelineSegment {
            iteration_id: it.id.clone(),
            title: it.title.clone(),
            start_date: it.start_date.clone(),
            duration: it.duration,
            completed: it.completed,
            contains_today,
            start_col: cursor,
            width: w,
        });
        cursor = cursor.saturating_add(w);
    }
    segments
}

/// YYYY-MM-DD 形式をパースして (year, month, day) を返す。
pub fn parse_ymd(s: &str) -> Option<(i32, u32, u32)> {
    if s.len() != 10 || s.as_bytes()[4] != b'-' || s.as_bytes()[7] != b'-' {
        return None;
    }
    let y: i32 = s[0..4].parse().ok()?;
    let m: u32 = s[5..7].parse().ok()?;
    let d: u32 = s[8..10].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

fn date_to_days(s: &str) -> Option<i64> {
    let (y, m, d) = parse_ymd(s)?;
    Some(days_from_civil(y, m, d))
}

/// Howard Hinnant's proleptic Gregorian "days from civil" algorithm。
/// 1970-01-01 からの経過日数を返す (epoch より前は負)。
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64;
    let m_adj = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * m_adj as i64 + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era as i64 * 146097 + doe - 719468
}

/// Hinnant's inverse (days since 1970-01-01 → (y, m, d))。
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y_raw = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y_raw + 1 } else { y_raw };
    (y as i32, m as u32, d as u32)
}

/// 現在日付 (UTC) を (year, month, day) で返す。システム時刻が取得できない場合は None。
pub fn today_utc() -> Option<(i32, u32, u32)> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    let days = (elapsed.as_secs() / 86400) as i64;
    Some(civil_from_days(days))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iteration(id: &str, title: &str, start_date: &str, duration: i32, completed: bool) -> IterationOption {
        IterationOption {
            id: id.into(),
            title: title.into(),
            start_date: start_date.into(),
            duration,
            completed,
        }
    }

    #[test]
    fn empty_iterations_yield_empty_segments() {
        let out = roadmap_timeline(&[], Some((2026, 4, 16)), 80);
        assert!(out.is_empty());
    }

    #[test]
    fn zero_width_yields_empty_segments() {
        let its = vec![iteration("it1", "S1", "2026-04-01", 14, false)];
        let out = roadmap_timeline(&its, Some((2026, 4, 8)), 0);
        assert!(out.is_empty());
    }

    #[test]
    fn window_picks_up_to_5_iterations_around_today() {
        // 7 iterations (7 日刻み、1 月にまたがらないよう短く)。
        // start: 01, 08, 15, 22, 29, 06(次月), 13(次月)
        let its: Vec<IterationOption> = vec![
            iteration("it0", "S0", "2026-04-01", 7, false),
            iteration("it1", "S1", "2026-04-08", 7, false),
            iteration("it2", "S2", "2026-04-15", 7, false),
            iteration("it3", "S3", "2026-04-22", 7, false),
            iteration("it4", "S4", "2026-04-29", 7, false),
            iteration("it5", "S5", "2026-05-06", 7, false),
            iteration("it6", "S6", "2026-05-13", 7, false),
        ];
        // today = 2026-04-17 (it2 の中に含まれる → center=2 → window=[0..5])
        let out = roadmap_timeline(&its, Some((2026, 4, 17)), 100);
        assert_eq!(out.len(), 5);
        assert_eq!(out[0].iteration_id, "it0");
        assert_eq!(out[4].iteration_id, "it4");
        assert!(out.iter().any(|s| s.iteration_id == "it2" && s.contains_today));
    }

    #[test]
    fn widths_sum_to_total_width() {
        let its: Vec<IterationOption> = vec![
            iteration("a", "A", "2026-04-01", 7, false),
            iteration("b", "B", "2026-04-08", 14, false),
            iteration("c", "C", "2026-04-22", 7, false),
        ];
        let out = roadmap_timeline(&its, Some((2026, 4, 10)), 80);
        let total: u16 = out.iter().map(|s| s.width).sum();
        assert_eq!(total, 80);
    }

    #[test]
    fn today_before_all_iterations_picks_first() {
        let its = vec![
            iteration("a", "A", "2026-05-01", 14, false),
            iteration("b", "B", "2026-05-15", 14, false),
        ];
        let out = roadmap_timeline(&its, Some((2026, 4, 1)), 60);
        // today は全 iteration より前なので「最初の today 以降」= a を center に
        assert_eq!(out.first().map(|s| s.iteration_id.as_str()), Some("a"));
        assert!(!out.iter().any(|s| s.contains_today));
    }

    #[test]
    fn today_none_picks_first_non_completed() {
        let its = vec![
            iteration("a", "A", "2026-03-01", 14, true),
            iteration("b", "B", "2026-03-15", 14, true),
            iteration("c", "C", "2026-03-29", 14, false),
            iteration("d", "D", "2026-04-12", 14, false),
        ];
        let out = roadmap_timeline(&its, None, 60);
        // non-completed の最初は c (index=2)、window は [0,1,2,3] (center=2, win=[0..4])
        assert!(out.iter().any(|s| s.iteration_id == "c"));
    }

    #[test]
    fn segments_are_contiguous() {
        let its = vec![
            iteration("a", "A", "2026-04-01", 7, false),
            iteration("b", "B", "2026-04-08", 7, false),
        ];
        let out = roadmap_timeline(&its, Some((2026, 4, 5)), 40);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].start_col, 0);
        assert_eq!(out[1].start_col, out[0].width);
    }

    #[test]
    fn days_from_civil_known_values() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1970, 1, 2), 1);
        assert_eq!(days_from_civil(1969, 12, 31), -1);
        // 2020-01-01 = 18262 days since epoch
        assert_eq!(days_from_civil(2020, 1, 1), 18262);
    }

    #[test]
    fn civil_from_days_roundtrip() {
        for (y, m, d) in [(1970, 1, 1), (2020, 2, 29), (2026, 4, 16), (1999, 12, 31)] {
            let days = days_from_civil(y, m, d);
            assert_eq!(civil_from_days(days), (y, m, d), "roundtrip for {y}-{m}-{d}");
        }
    }

    #[test]
    fn parse_ymd_rejects_invalid() {
        assert!(parse_ymd("2026-13-01").is_none());
        assert!(parse_ymd("2026-04-32").is_none());
        assert!(parse_ymd("not-a-date").is_none());
        assert_eq!(parse_ymd("2026-04-16"), Some((2026, 4, 16)));
    }
}
