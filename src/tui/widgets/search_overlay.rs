use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    // Centered popup: 80% wide, 50% tall
    let popup = centered_rect(80, 60, area);

    // Clear background
    f.render_widget(ratatui::widgets::Clear, popup);

    let block = Block::default()
        .title(" Search (↑↓ navigate, Enter open, Esc close) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    // Query input
    let query_line = Line::from(vec![
        Span::styled("/ ", Style::default().fg(Color::Cyan)),
        Span::raw(state.search_query.clone()),
        Span::styled("█", Style::default().fg(Color::Cyan)),
    ]);
    f.render_widget(Paragraph::new(query_line), chunks[0]);

    // Results list
    let items: Vec<ListItem> = state
        .search_results
        .iter()
        .map(|r| {
            ListItem::new(Line::from(vec![
                Span::styled(r.title.clone(), Style::default().fg(Color::White)),
                Span::styled(
                    format!("  {:.2}", r.score),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    if !state.search_results.is_empty() {
        list_state.select(Some(state.search_cursor));
    }

    f.render_stateful_widget(list, chunks[1], &mut list_state);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
