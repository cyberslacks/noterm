use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::search_overlay::centered_rect;
use crate::app::{AppState, Mode};

pub fn render(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let popup = centered_rect(80, 75, area);
    f.render_widget(ratatui::widgets::Clear, popup);

    let block = Block::default()
        .title(" Meetily Meetings  [Enter] import · [j/k] navigate · [Esc] close ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    if state.meetily.loading {
        let loading = Paragraph::new("Loading meetings from Meetily database…")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(loading, inner);
        return;
    }

    if state.meetily.meetings.is_empty() {
        let msg = Paragraph::new(
            "No meetings found.\n\nCheck that Meetily is installed and has recorded at least one meeting.\nAuto-detect locations: ~/.local/share/meetily/ · /opt/homebrew/var/meetily/"
        )
        .style(Style::default().fg(Color::DarkGray))
        .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(msg, inner);
        return;
    }

    let items: Vec<ListItem> = state
        .meetily
        .meetings
        .iter()
        .map(|m| {
            let imported = state.meetily.imported_ids.contains(&m.id);
            let date = m.date_display();
            let dur = m.duration_display();
            let dur_str = if dur.is_empty() {
                String::new()
            } else {
                format!("  {dur}")
            };
            let badge = if imported {
                Span::styled("  ✓ imported", Style::default().fg(Color::Green))
            } else {
                Span::raw("")
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{date}{dur_str}  "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(m.title.clone(), Style::default().fg(Color::White)),
                badge,
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.meetily.cursor));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, inner, &mut list_state);
}
