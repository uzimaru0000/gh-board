use ratatui::layout::{Constraint, Flex, Layout, Rect};

use crate::ui::board::COLUMN_WIDTH;

/// コンパクトレイアウトと判定する高さの下限 (行数)。
/// 80%×80% のモーダルが詳細を出すのに手狭になる境目。
pub const COMPACT_HEIGHT: u16 = 24;

/// 端末サイズが小さく、モーダルや複数カラムでは窮屈になる状態かどうか。
/// カンバンのカラムが 1 つしか収まらない幅、もしくは高さが極端に低い場合に true。
pub fn is_compact(area: Rect) -> bool {
    area.width < COLUMN_WIDTH * 2 || area.height < COMPACT_HEIGHT
}

/// モーダル/ポップアップの描画領域 (パーセント指定版)。
/// compact な端末サイズでは親 area を丸ごと使い (全画面化)、それ以外では
/// 指定パーセントの中央寄せ矩形を返す。
pub fn modal_area_pct(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    if is_compact(area) {
        area
    } else {
        centered_rect_pct(percent_x, percent_y, area)
    }
}

/// モーダル/ポップアップの描画領域 (固定高さ版)。
/// compact な端末サイズでは親 area を丸ごと使い (全画面化)、それ以外では
/// 指定パーセント幅 × 固定高さの中央寄せ矩形を返す。
pub fn modal_area_fixed(percent_x: u16, height: u16, area: Rect) -> Rect {
    if is_compact(area) {
        area
    } else {
        centered_rect_fixed(percent_x, height, area)
    }
}

pub fn centered_rect_pct(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}

pub fn centered_rect_fixed(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: u16, h: u16) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: w,
            height: h,
        }
    }

    #[test]
    fn is_compact_narrow_width() {
        // カラム 1 つしか収まらない幅 (COLUMN_WIDTH * 2 未満) は compact
        assert!(is_compact(rect(COLUMN_WIDTH * 2 - 1, 40)));
        assert!(is_compact(rect(COLUMN_WIDTH, 40)));
    }

    #[test]
    fn is_compact_wide_enough() {
        // 2 カラム分以上の幅かつ十分な高さなら compact ではない
        assert!(!is_compact(rect(COLUMN_WIDTH * 2, COMPACT_HEIGHT)));
        assert!(!is_compact(rect(120, 40)));
    }

    #[test]
    fn is_compact_short_height() {
        // 高さが極端に低い場合は compact
        assert!(is_compact(rect(120, COMPACT_HEIGHT - 1)));
        assert!(is_compact(rect(120, 10)));
    }

    #[test]
    fn modal_area_pct_compact_is_fullscreen() {
        let area = rect(50, 20);
        assert_eq!(modal_area_pct(80, 80, area), area);
        assert_eq!(modal_area_pct(60, 60, area), area);
    }

    #[test]
    fn modal_area_pct_wide_returns_centered() {
        let area = rect(100, 40);
        let popup = modal_area_pct(80, 80, area);
        assert_ne!(popup, area);
        assert_eq!(popup.width, 80);
        assert_eq!(popup.height, 32);
    }

    #[test]
    fn modal_area_fixed_compact_is_fullscreen() {
        let area = rect(50, 20);
        // compact のときは固定高さ指定も無視して全画面
        assert_eq!(modal_area_fixed(60, 10, area), area);
    }

    #[test]
    fn modal_area_fixed_wide_uses_fixed_height() {
        let area = rect(100, 40);
        let popup = modal_area_fixed(60, 10, area);
        assert_ne!(popup, area);
        assert_eq!(popup.width, 60);
        assert_eq!(popup.height, 10);
    }
}
