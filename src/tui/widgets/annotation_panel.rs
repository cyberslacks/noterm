use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::search_overlay::centered_rect;
use crate::app::AppState;
use crate::notes::annotations::AnnotationStatus;

pub fn render(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let popup = centered_rect(75, 70, area);
    f.render_widget(ratatui::widgets::Clear, popup);

    let block = Block::default()
        .title(" Annotations  [n] new · [i] incorporate · [d] ignore · [j/k] nav · [Esc] close ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    // Reserve bottom row for compose input when composing
    let (list_area, compose_area) = if state.annotation.composing {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(5)])
            .split(inner);
        (chunks[0], Some(chunks[1]))
    } else {
        (inner, None)
    };

    if state.annotation.entries.is_empty() && !state.annotation.composing {
        let msg = Paragraph::new(
            "No annotations for this note.\n\n\
             Press [n] to add a new annotation.",
        )
        .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, list_area);
    } else {
        let items: Vec<ListItem> = state
            .annotation
            .entries
            .iter()
            .map(|ann| {
                let (label, color) = status_badge(&ann.status);
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<16}", label),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  [{}]", ann.added),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(format!("  {}", ann.text), Style::default().fg(Color::White)),
                ]))
            })
            .collect();

        let mut list_state = ListState::default();
        if !state.annotation.entries.is_empty() {
            list_state.select(Some(state.annotation.cursor));
        }

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, list_area, &mut list_state);
    }

    if let Some(compose) = compose_area {
        let compose_block = Block::default()
            .title(" New annotation  (Enter to save · Esc to cancel) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner_compose = compose_block.inner(compose);
        f.render_widget(compose_block, compose);

        let cursor_line = Line::from(vec![
            Span::raw("  "),
            Span::raw(state.annotation.input.clone()),
            Span::styled("█", Style::default().fg(Color::Cyan)),
        ]);
        f.render_widget(Paragraph::new(cursor_line), inner_compose);
    }
}

fn status_badge(status: &AnnotationStatus) -> (&'static str, Color) {
    match status {
        AnnotationStatus::Pending => ("PENDING", Color::Yellow),
        AnnotationStatus::Incorporated => ("INCORPORATED", Color::Green),
        AnnotationStatus::Ignored => ("IGNORED", Color::DarkGray),
        AnnotationStatus::Stale => ("STALE", Color::Red),
    }
}
