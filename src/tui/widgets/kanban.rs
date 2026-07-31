use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

use crate::app::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let col_count = state.kanban.columns.len();
    if col_count == 0 {
        return;
    }

    let constraints: Vec<Constraint> = (0..col_count)
        .map(|_| Constraint::Percentage(100 / col_count as u16))
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (i, (col, chunk)) in state.kanban.columns.iter().zip(chunks.iter()).enumerate() {
        let is_focused = i == state.kanban.focused_col;
        let border_style = if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .title(format!(" {} ({}) ", col.status.label(), col.cards.len()))
            .borders(Borders::ALL)
            .border_type(if is_focused {
                BorderType::Double
            } else {
                BorderType::Rounded
            })
            .border_style(border_style);

        let items: Vec<ListItem> = col
            .cards
            .iter()
            .enumerate()
            .map(|(j, task)| {
                let priority_icon = match task.priority {
                    Some(1) => "🔴 ",
                    Some(2) => "🟠 ",
                    Some(3) => "🟡 ",
                    _ => "   ",
                };
                let style = if is_focused && j == state.kanban.focused_card {
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                };
                ListItem::new(Line::from(vec![
                    Span::raw(priority_icon.to_string()),
                    Span::styled(task.title.clone(), style),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::DarkGray));

        let mut list_state = ListState::default();
        if is_focused && !col.cards.is_empty() {
            list_state.select(Some(state.kanban.focused_card));
        }

        f.render_stateful_widget(list, *chunk, &mut list_state);
    }
}
