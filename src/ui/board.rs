use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
    Frame,
};

use crate::app::App;
use crate::app_state::AppState;
use crate::model::state::ViewMode;
use crate::ui::card::{CardWidget, CARD_HEIGHT};
use crate::ui::scroll_fade::{draw_bottom_arrow, draw_left_arrow, draw_right_arrow, draw_top_arrow};
use crate::ui::statusline::loading_spinner_span;
use crate::ui::theme::theme;

pub const COLUMN_WIDTH: u16 = 36;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let board = match &app.state.board {
        Some(b) => b,
        None => return,
    };

    if board.columns.is_empty() {
        return;
    }

    // 全体に padding を持たせる
    let area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let num_cols = board.columns.len();
    let visible_cols = (area.width / COLUMN_WIDTH).max(1) as usize;

    // 横スクロール: 選択カラムが常に表示されるように調整
    let scroll_x = AppState::compute_board_scroll_x(
        app.state.selected_column,
        app.state.board_scroll_x.get(),
        visible_cols,
        num_cols,
    );
    app.state.board_scroll_x.set(scroll_x);

    let end = (scroll_x + visible_cols).min(num_cols);
    let render_count = end - scroll_x;

    let constraints: Vec<Constraint> = (0..render_count)
        .map(|_| Constraint::Length(COLUMN_WIDTH))
        .collect();

    let col_areas = Layout::horizontal(constraints).split(area);

    for (vis_idx, col_idx) in (scroll_x..end).enumerate() {
        let column = &board.columns[col_idx];
        let col_area = col_areas[vis_idx];
        let is_selected_col = col_idx == app.state.selected_column;

        let column_fg = column.color.as_ref().map(|c| theme().column_color(c));

        let title_style = if is_selected_col {
            let style = Style::default().add_modifier(Modifier::BOLD);
            match column_fg {
                Some(c) => style.fg(c),
                None => style.fg(theme().accent),
            }
        } else {
            match column_fg {
                Some(c) => Style::default().fg(c),
                None => Style::default().fg(theme().text),
            }
        };

        let border_style = if is_selected_col {
            Style::default().fg(theme().border_focused)
        } else {
            Style::default().fg(theme().border_unfocused)
        };

        // フィルタ適用: フィルタに一致するカードのインデックスを収集
        let filtered_indices: Vec<usize> = column
            .cards
            .iter()
            .enumerate()
            .filter(|(_, card)| app.state.filter.active_filter.as_ref().is_none_or(|f| f.matches(card)))
            .map(|(idx, _)| idx)
            .collect();

        let total = filtered_indices.len();

        // max_visible は Block::inner 計算後に確定するが、title 構築には先に仮算出する必要はない。
        // 一旦 block を作ってから inner を求め、スクロール情報を含めた title に置き換える。
        let tmp_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        let inner = tmp_block.inner(col_area);

        let max_visible = (inner.height / CARD_HEIGHT) as usize;

        // Calculate scroll offset for this column
        let scroll = if is_selected_col && !filtered_indices.is_empty() && max_visible > 0 {
            let selected = app.state.selected_card.min(filtered_indices.len().saturating_sub(1));
            if selected >= app.state.scroll_offset + max_visible {
                selected - max_visible + 1
            } else if selected < app.state.scroll_offset {
                selected
            } else {
                app.state.scroll_offset
            }
        } else {
            0
        };

        // タイトル: スクロール可能なら "N-M/Total"、そうでなければ "(Total)"
        let title_text = if AppState::should_show_scrollbar(total, max_visible) {
            let start = scroll + 1;
            let end = (scroll + max_visible).min(total);
            format!(" {} {start}-{end}/{total} ", column.name)
        } else {
            format!(" {} ({}) ", column.name, total)
        };
        let mut title_spans: Vec<Span> = vec![Span::styled(title_text, title_style)];
        if let Some(spinner) = loading_spinner_span(&app.state.loading) {
            title_spans.push(spinner);
        }
        let col_block = Block::default()
            .title(Line::from(title_spans))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style);

        frame.render_widget(col_block, col_area);

        if max_visible == 0 || filtered_indices.is_empty() {
            continue;
        }

        let visible_cards = filtered_indices
            .iter()
            .enumerate()
            .skip(scroll)
            .take(max_visible);

        let mut grab_shadow_area: Option<Rect> = None;

        for (i, (display_idx, &card_idx)) in visible_cards.enumerate() {
            let y = inner.y + (i as u16 * CARD_HEIGHT);
            if y + CARD_HEIGHT > inner.y + inner.height {
                break;
            }

            let card_area = Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: CARD_HEIGHT,
            };

            let selected = is_selected_col && display_idx == app.state.selected_card;
            let grabbing = app.state.mode == ViewMode::CardGrab && selected;

            frame.render_widget(
                CardWidget {
                    card: &column.cards[card_idx],
                    selected,
                    grabbing,
                },
                card_area,
            );

            if grabbing {
                grab_shadow_area = Some(card_area);
            }
        }

        // 影は全カード描画後に描画（次のカードに上書きされないように）
        if let Some(area) = grab_shadow_area {
            render_shadow(frame.buffer_mut(), area);
        }

        // カラム上下のボーダー中央に矢印を配置 (画面外にカードがあるときのみ)
        let has_above = scroll > 0;
        let has_below = scroll + max_visible < filtered_indices.len();
        let buf = frame.buffer_mut();
        if has_above {
            draw_top_arrow(buf, col_area);
        }
        if has_below {
            draw_bottom_arrow(buf, col_area);
        }
    }

    // 横方向の矢印: カラムが左右に隠れている方向のボーダー中央に描画
    let has_left = scroll_x > 0;
    let has_right = end < num_cols;
    let buf = frame.buffer_mut();
    if has_left {
        draw_left_arrow(buf, area);
    }
    if has_right {
        draw_right_arrow(buf, area);
    }
}

fn render_shadow(buf: &mut Buffer, card_area: Rect) {
    // 既存セルの文字を残しつつ色を暗くして透過風の影にする
    let shadow_fg = theme().shadow_fg;
    let shadow_bg = theme().shadow_bg;

    let dim = |buf: &mut Buffer, x: u16, y: u16| {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_fg(shadow_fg);
            cell.set_bg(shadow_bg);
        }
    };

    // 右辺の影
    let shadow_x = card_area.x + card_area.width;
    for dy in 1..card_area.height {
        dim(buf, shadow_x, card_area.y + dy);
    }

    // 下辺の影
    let shadow_y = card_area.y + card_area.height;
    for dx in 1..=card_area.width {
        dim(buf, card_area.x + dx, shadow_y);
    }
}
