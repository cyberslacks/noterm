use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::search_overlay::centered_rect;
use crate::app::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let popup = centered_rect(80, 60, area);
    f.render_widget(ratatui::widgets::Clear, popup);

    let block = Block::default()
        .title(" Semantic Search (↑↓ navigate, Enter open, Esc close) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    // Query input
    let query_line = Line::from(vec![
        Span::styled("~ ", Style::default().fg(Color::Magenta)),
        Span::raw(state.vsearch_query.clone()),
        Span::styled("█", Style::default().fg(Color::Magenta)),
    ]);
    f.render_widget(Paragraph::new(query_line), chunks[0]);

    // Loading indicator
    if state.vsearch_loading {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  Searching...",
                Style::default().fg(Color::DarkGray),
            )),
            chunks[1],
        );
    }

    // Results
    let items: Vec<ListItem> = state
        .vsearch_results
        .iter()
        .map(|r| {
            let sim_pct = (r.similarity * 100.0) as u8;
            ListItem::new(Line::from(vec![
                Span::styled(r.relative_path.clone(), Style::default().fg(Color::White)),
                Span::styled(
                    format!("  {sim_pct}%"),
                    Style::default().fg(similarity_color(r.similarity)),
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
    if !state.vsearch_results.is_empty() {
        list_state.select(Some(state.vsearch_cursor));
    }
    f.render_stateful_widget(list, chunks[2], &mut list_state);
}

fn similarity_color(sim: f32) -> Color {
    if sim > 0.85 {
        Color::Green
    } else if sim > 0.7 {
        Color::Yellow
    } else {
        Color::DarkGray
    }
}
