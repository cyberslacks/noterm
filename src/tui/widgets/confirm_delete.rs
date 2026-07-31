use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use super::search_overlay::centered_rect;
use crate::app::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let popup = centered_rect(55, 20, area);
    f.render_widget(ratatui::widgets::Clear, popup);

    let block = Block::default()
        .title(" Delete Note ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    // What will be deleted
    let filename = state
        .selected_file_node()
        .map(|n| n.name.clone())
        .unwrap_or_else(|| "?".into());

    let file_line = Line::from(vec![
        Span::raw("  Delete: "),
        Span::styled(
            filename,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(file_line), chunks[0]);

    // Confirm prompt
    let prompt = Line::from(vec![
        Span::styled(
            "  [y] ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw("Confirm delete    "),
        Span::styled("[n / Esc] ", Style::default().fg(Color::Green)),
        Span::raw("Cancel"),
    ]);
    f.render_widget(Paragraph::new(prompt), chunks[1]);
}
