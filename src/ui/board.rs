use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
    Frame,
};

use crate::app::App;
use crate::ui::card::{CardWidget, CARD_HEIGHT};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let board = match &app.state.board {
        Some(b) => b,
        None => return,
    };

    if board.columns.is_empty() {
        return;
    }

    let num_cols = board.columns.len() as u32;
    let constraints: Vec<Constraint> = (0..num_cols)
        .map(|_| Constraint::Ratio(1, num_cols))
        .collect();

    let col_areas = Layout::horizontal(constraints).split(area);

    for (col_idx, (column, &col_area)) in
        board.columns.iter().zip(col_areas.iter()).enumerate()
    {
        let is_selected_col = col_idx == app.state.selected_column;

        let title_style = if is_selected_col {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let border_style = if is_selected_col {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // フィルタ適用: フィルタに一致するカードのインデックスを収集
        let filtered_indices: Vec<usize> = column
            .cards
            .iter()
            .enumerate()
            .filter(|(_, card)| app.state.filter.active_filter.as_ref().map_or(true, |f| f.matches(card)))
            .map(|(idx, _)| idx)
            .collect();

        let title = format!(" {} ({}) ", column.name, filtered_indices.len());
        let col_block = Block::default()
            .title(title)
            .title_style(title_style)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = col_block.inner(col_area);
        frame.render_widget(col_block, col_area);

        // Render cards within this column
        let max_visible = (inner.height / CARD_HEIGHT) as usize;
        if max_visible == 0 || filtered_indices.is_empty() {
            continue;
        }

        // Calculate scroll offset for this column
        let scroll = if is_selected_col {
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

        let visible_cards = filtered_indices
            .iter()
            .enumerate()
            .skip(scroll)
            .take(max_visible);

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
            frame.render_widget(
                CardWidget {
                    card: &column.cards[card_idx],
                    selected,
                },
                card_area,
            );
        }
    }
}
