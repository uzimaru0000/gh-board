use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};

use crate::ui::theme::theme;

/// `area` の上端右寄りに上方向スクロール可能を示す ▲ 印を描く。
/// 右上角の 2 セル内側に配置し、タイトル (左寄せ) と被らないようにする。
pub fn draw_top_arrow(buf: &mut Buffer, area: Rect) {
    if area.width < 3 || area.height == 0 {
        return;
    }
    let y = area.y;
    let x = area.x + area.width - 2;
    set_arrow(buf, x, y, "▲");
}

/// `area` の下端右寄りに下方向スクロール可能を示す ▼ 印を描く。
pub fn draw_bottom_arrow(buf: &mut Buffer, area: Rect) {
    if area.width < 3 || area.height == 0 {
        return;
    }
    let y = area.y + area.height - 1;
    let x = area.x + area.width - 2;
    set_arrow(buf, x, y, "▼");
}

/// `area` の左端中央に左方向スクロール可能を示す ◀ 印を描く。
pub fn draw_left_arrow(buf: &mut Buffer, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let x = area.x;
    let y = area.y + area.height / 2;
    set_arrow(buf, x, y, "◀");
}

/// `area` の右端中央に右方向スクロール可能を示す ▶ 印を描く。
pub fn draw_right_arrow(buf: &mut Buffer, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let x = area.x + area.width - 1;
    let y = area.y + area.height / 2;
    set_arrow(buf, x, y, "▶");
}

fn set_arrow(buf: &mut Buffer, x: u16, y: u16, symbol: &str) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(symbol);
        cell.set_style(
            Style::default()
                .fg(theme().accent)
                .add_modifier(Modifier::BOLD),
        );
    }
}
