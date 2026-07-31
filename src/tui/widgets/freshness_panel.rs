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
        .title(" Freshness Review  [Enter] open · [j/k] navigate · [Esc] close ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    if state.freshness.loading {
        let loading = Paragraph::new("Scanning notes for freshness metadata…")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(loading, chunks[0]);
        return;
    }

    if state.freshness.entries.is_empty() {
        let msg = Paragraph::new(
            "No notes with freshness metadata found.\n\n\
             Add `review_every: 30d` to a note's YAML frontmatter to track staleness.\n\
             Supported: Nd · Nw · Nm · Ny · weekly · monthly · quarterly · yearly",
        )
        .style(Style::default().fg(Color::DarkGray))
        .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(msg, chunks[0]);
        return;
    }

    let items: Vec<ListItem> = state
        .freshness
        .entries
        .iter()
        .map(|entry| {
            let (badge_text, badge_color) = status_badge(entry.status);
            let owner_str = entry
                .owner
                .as_deref()
                .map(|o| format!("  {o}"))
                .unwrap_or_default();
            let cadence = entry
                .review_every
                .as_deref()
                .map(|r| format!("  [{r}]"))
                .unwrap_or_default();

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<20}", badge_text),
                    Style::default()
                        .fg(badge_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    entry.relative_path.clone(),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("  {}", entry.title),
                    Style::default().fg(Color::White),
                ),
                Span::styled(cadence, Style::default().fg(Color::DarkGray)),
                Span::styled(owner_str, Style::default().fg(Color::Cyan)),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.freshness.cursor));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[0], &mut list_state);

    // Summary counts
    let (expired, overdue, due_soon, fresh) = count_statuses(&state.freshness.entries);
    let summary =
        format!(" expired:{expired}  overdue:{overdue}  due soon:{due_soon}  fresh:{fresh}",);
    f.render_widget(
        Paragraph::new(summary).style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );
}

fn status_badge(status: FreshnessStatus) -> (String, Color) {
    match status {
        FreshnessStatus::Expired { days_past_expiry } => {
            (format!("EXPIRED {days_past_expiry}d"), Color::Red)
        }
        FreshnessStatus::Overdue { days_overdue } => {
            (format!("OVERDUE {days_overdue}d"), Color::Red)
        }
        FreshnessStatus::DueSoon { days_until_due } => {
            (format!("DUE IN {days_until_due}d"), Color::Yellow)
        }
        FreshnessStatus::Fresh => ("FRESH".to_string(), Color::Green),
    }
}

fn count_statuses(
    entries: &[crate::notes::freshness::FreshnessEntry],
) -> (usize, usize, usize, usize) {
    let mut expired = 0;
    let mut overdue = 0;
    let mut due_soon = 0;
    let mut fresh = 0;
    for e in entries {
        match e.status {
            FreshnessStatus::Expired { .. } => expired += 1,
            FreshnessStatus::Overdue { .. } => overdue += 1,
            FreshnessStatus::DueSoon { .. } => due_soon += 1,
            FreshnessStatus::Fresh => fresh += 1,
        }
    }
    (expired, overdue, due_soon, fresh)
}
