use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::{AppState, Mode, StatusLevel};
use crate::notes::freshness::{self, FreshnessStatus};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    // Mode indicator
    let (mode_str, mode_color) = match state.mode {
        Mode::Normal => ("NORMAL", Color::Green),
        Mode::Edit => ("INSERT", Color::Yellow),
        Mode::Search => ("SEARCH", Color::Cyan),
        Mode::VectorSearch => ("VSEARCH", Color::Magenta),
        Mode::Chat => ("CHAT", Color::Blue),
        Mode::Kanban => ("KANBAN", Color::LightGreen),
        Mode::Git => ("GIT", Color::LightRed),
        Mode::Help => ("HELP", Color::White),
        Mode::NewNote => ("NEW NOTE", Color::Yellow),
        Mode::NewCollection => ("NEW COLLECTION", Color::Yellow),
        Mode::GitCommitInput => ("COMMIT", Color::Yellow),
        Mode::ConfirmDelete => ("DELETE?", Color::Red),
        Mode::MeetilyImport => ("MEETILY", Color::Cyan),
        Mode::Settings => ("SETTINGS", Color::Cyan),
        Mode::Summarize => ("SUMMARIZE", Color::LightMagenta),
        Mode::FreshnessView => ("FRESHNESS", Color::Yellow),
        Mode::AnnotationPanel => ("ANNOTATE", Color::Yellow),
        Mode::KazamKbBrowser => ("KAZAM KB", Color::Cyan),
        Mode::VaultPicker => ("VAULTS", Color::Cyan),
    };

    // Compute freshness badge for the open note (if it has review_every)
    let freshness_span = state.current_note.as_ref().and_then(|note| {
        let fm = &note.frontmatter;
        let info = freshness::compute(
            fm.modified.as_deref(),
            fm.review_every.as_deref(),
            fm.expires.as_deref(),
        )?;
        let (label, color) = match info.status() {
            FreshnessStatus::Expired { days_past_expiry } => {
                (format!(" EXPIRED {days_past_expiry}d "), Color::Red)
            }
            FreshnessStatus::Overdue { days_overdue } => {
                (format!(" OVERDUE {days_overdue}d "), Color::Red)
            }
            FreshnessStatus::DueSoon { days_until_due } => {
                (format!(" DUE IN {days_until_due}d "), Color::Yellow)
            }
            FreshnessStatus::Fresh => (" FRESH ".to_string(), Color::Green),
        };
        Some(Span::styled(
            label,
            Style::default()
                .fg(Color::Black)
                .bg(color)
                .add_modifier(Modifier::BOLD),
        ))
    });

    let mut left_spans = vec![
        Span::styled(
            format!(" {mode_str} "),
            Style::default()
                .fg(Color::Black)
                .bg(mode_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            state
                .current_note
                .as_ref()
                .map(|n| n.relative_path.clone())
                .unwrap_or_else(|| state.notes_dir.to_string_lossy().to_string()),
            Style::default().fg(Color::White),
        ),
    ];
    if let Some(badge) = freshness_span {
        left_spans.push(Span::raw(" "));
        left_spans.push(badge);
    }

    // Annotation pending count badge
    if state.annotation_pending_count > 0 && state.current_note.is_some() {
        left_spans.push(Span::raw(" "));
        left_spans.push(Span::styled(
            format!(" A:{} ", state.annotation_pending_count),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let left = Line::from(left_spans);

    let right_text = if let Some((msg, level)) = &state.status_message {
        let color = match level {
            StatusLevel::Info => Color::White,
            StatusLevel::Success => Color::Green,
            StatusLevel::Warning => Color::Yellow,
            StatusLevel::Error => Color::Red,
        };
        Line::from(Span::styled(format!("{msg} "), Style::default().fg(color)))
    } else if let Some(version) = &state.update_available {
        Line::from(vec![Span::styled(
            format!(" \u{2b06} {version} available "),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )])
    } else {
        let branch = state.git_branch();
        Line::from(Span::styled(
            format!(" git:{branch} "),
            Style::default().fg(Color::DarkGray),
        ))
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(35)])
        .split(area);

    f.render_widget(
        Paragraph::new(left).style(Style::default().bg(Color::Black)),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(right_text).style(Style::default().bg(Color::Black)),
        chunks[1],
    );
}
