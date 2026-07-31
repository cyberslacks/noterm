use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};

use super::search_overlay::centered_rect;
use crate::app::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let popup = centered_rect(75, 70, area);
    f.render_widget(ratatui::widgets::Clear, popup);

    let outer = Block::default()
        .title(" Git (s=stage, c=commit, p=push, P=pull, Tab=switch, Esc=close) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::LightRed));

    let inner = outer.inner(popup);
    f.render_widget(outer, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(inner);

    // Tabs
    let tab_titles = ["Status", "Log"];
    let tabs = Tabs::new(tab_titles.map(|t| t.to_string()).to_vec())
        .select(state.git_selected_tab)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, chunks[0]);

    match state.git_selected_tab {
        0 => render_status(f, chunks[1], state),
        1 => render_log(f, chunks[1], state),
        _ => {}
    }
}

fn render_status(f: &mut Frame, area: Rect, state: &AppState) {
    if let Some(status) = &state.git_status {
        let mut items: Vec<ListItem> = Vec::new();

        let branch_line = ListItem::new(Line::from(vec![
            Span::styled("Branch: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                status.branch.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        items.push(branch_line);
        items.push(ListItem::new(""));

        if !status.staged.is_empty() {
            items.push(ListItem::new(Span::styled(
                "Staged:",
                Style::default().fg(Color::Green),
            )));
            for f in &status.staged {
                items.push(ListItem::new(format!("  + {f}")));
            }
        }
        if !status.unstaged.is_empty() {
            items.push(ListItem::new(Span::styled(
                "Modified:",
                Style::default().fg(Color::Yellow),
            )));
            for f in &status.unstaged {
                items.push(ListItem::new(format!("  ~ {f}")));
            }
        }
        if !status.untracked.is_empty() {
            items.push(ListItem::new(Span::styled(
                "Untracked:",
                Style::default().fg(Color::Red),
            )));
            for f in &status.untracked {
                items.push(ListItem::new(format!("  ? {f}")));
            }
        }

        f.render_widget(List::new(items), area);
    } else {
        f.render_widget(
            Paragraph::new("Loading git status...").style(Style::default().fg(Color::DarkGray)),
            area,
        );
    }
}

fn render_log(f: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .git_log
        .iter()
        .map(|c| {
            ListItem::new(Line::from(vec![
                Span::styled(c.hash.clone(), Style::default().fg(Color::Yellow)),
                Span::raw("  "),
                Span::raw(c.message.clone()),
                Span::styled(
                    format!("  — {} {}", c.author, c.time),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), area);
}
