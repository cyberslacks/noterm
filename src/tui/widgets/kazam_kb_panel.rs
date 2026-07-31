use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::search_overlay::centered_rect;
use crate::app::AppState;
use crate::notes::freshness::FreshnessStatus;

pub fn render(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let popup = centered_rect(85, 80, area);
    f.render_widget(ratatui::widgets::Clear, popup);

    let block = Block::default()
        .title(" Kazam KB  [Enter] import/open · [j/k] nav · (type) filter · [Esc] close ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(inner);

    if state.kazam_kb.loading {
        let loading =
            Paragraph::new("Scanning Kazam KB…").style(Style::default().fg(Color::DarkGray));
        f.render_widget(loading, chunks[0]);
        render_filter_input(f, chunks[1], state);
        return;
    }

    let filter = state.kazam_kb.filter.to_lowercase();
    let filtered: Vec<&crate::kazam::kb_browser::KazamPage> = state
        .kazam_kb
        .entries
        .iter()
        .filter(|p| {
            filter.is_empty()
                || p.title.to_lowercase().contains(&filter)
                || p.slug.contains(&filter)
        })
        .collect();

    if filtered.is_empty() {
        let msg = if state.kazam_kb.entries.is_empty() {
            "No Kazam KB pages found.\n\nSet kazam.kb_path in settings (S key)."
        } else {
            "No pages match the current filter."
        };
        f.render_widget(
            Paragraph::new(msg).style(Style::default().fg(Color::DarkGray)),
            chunks[0],
        );
        render_filter_input(f, chunks[1], state);
        return;
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|page| {
            let (status_str, status_color) = freshness_badge(page.freshness_status);
            let imported_tag = if page.already_imported {
                Span::styled(" [imported]", Style::default().fg(Color::Green))
            } else {
                Span::raw("")
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<16}", status_str),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<30}", page.slug),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(page.title.clone(), Style::default().fg(Color::White)),
                imported_tag,
                Span::styled(
                    page.owner
                        .as_deref()
                        .map(|o| format!("  {o}"))
                        .unwrap_or_default(),
                    Style::default().fg(Color::Cyan),
                ),
            ]))
        })
        .collect();

    let cursor = state.kazam_kb.cursor.min(filtered.len().saturating_sub(1));
    let mut list_state = ListState::default();
    list_state.select(Some(cursor));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[0], &mut list_state);
    render_filter_input(f, chunks[1], state);
}

fn render_filter_input(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let block = Block::default()
        .title(" Filter ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let line = Line::from(vec![
        Span::raw("  "),
        Span::raw(state.kazam_kb.filter.clone()),
        Span::styled("█", Style::default().fg(Color::Cyan)),
    ]);
    f.render_widget(Paragraph::new(line), inner);
}

fn freshness_badge(status: Option<FreshnessStatus>) -> (&'static str, Color) {
    match status {
        None => ("", Color::DarkGray),
        Some(FreshnessStatus::Fresh) => ("FRESH", Color::Green),
        Some(FreshnessStatus::DueSoon { .. }) => ("DUE SOON", Color::Yellow),
        Some(FreshnessStatus::Overdue { .. }) => ("OVERDUE", Color::Red),
        Some(FreshnessStatus::Expired { .. }) => ("EXPIRED", Color::Red),
    }
}
