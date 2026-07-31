use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use super::search_overlay::centered_rect;
use crate::app::AppState;

pub fn render(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let popup = centered_rect(70, 85, area);
    f.render_widget(ratatui::widgets::Clear, popup);

    let block = Block::default()
        .title(" Help  [j/k] scroll · [any other key] close ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::White));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let bindings: &[(&str, &str)] = &[
        ("Navigation", ""),
        ("j / ↓", "Move down in file tree"),
        ("k / ↑", "Move up in file tree"),
        ("Enter", "Open selected file"),
        ("Tab", "Cycle file grouping: None → Modified → Created"),
        ("PageUp / PageDn", "Scroll note viewer"),
        ("", ""),
        ("Normal Mode", ""),
        ("e", "Edit current note (insert mode)"),
        ("n", "New note (enter filename)"),
        ("d", "Delete selected note (confirmation required)"),
        ("/", "Full-text search (Tantivy)"),
        ("v", "Vector / semantic search (embeddings)"),
        ("c", "Toggle LLM chat panel"),
        ("K", "Kanban board (tasks from note frontmatter)"),
        ("G", "Git panel (status, log, stage, commit, push)"),
        ("I", "Meetily import panel (call notes from DB)"),
        ("S", "Settings (LLM provider, models, API keys)"),
        ("X", "AI summarize current note → inserts into ## Summary"),
        ("F", "Freshness report (notes with review_every metadata)"),
        ("A", "Annotations panel (sidecar notes for current note)"),
        ("B", "Kazam KB browser (browse & import KB pages)"),
        ("E", "Export note → Kazam KB YAML (Track B)"),
        ("M", "Toggle Kazam MCP connection (Track B)"),
        ("?", "This help screen"),
        ("q / Ctrl+q", "Quit"),
        ("", ""),
        ("Editor (Insert mode)", ""),
        ("Ctrl+s", "Save note"),
        ("Esc", "Save and return to Normal"),
        ("", ""),
        ("Search", ""),
        ("(type)", "Filter notes by content"),
        ("Enter / ↑↓", "Jump to result"),
        ("Esc", "Close search"),
        ("", ""),
        ("Chat", ""),
        ("(type)", "Compose message"),
        ("Enter", "Send message to LLM"),
        ("Tab", "Toggle Kazam KB context in chat (Track B)"),
        ("Ctrl+l", "Clear chat history"),
        ("Esc", "Close chat panel"),
        ("", ""),
        ("Kanban", ""),
        ("h / l", "Move between columns"),
        ("j / k", "Navigate cards within column"),
        ("m", "Move focused card to next column"),
        ("Esc", "Close kanban"),
        ("", ""),
        ("Git", ""),
        ("Tab", "Switch Status / Log tabs"),
        ("s", "Stage all changes"),
        ("c", "Commit (opens message prompt)"),
        ("p", "Push to remote"),
        ("P", "Pull from remote"),
        ("Esc", "Close git panel"),
        ("", ""),
        ("Summarize (Shift+X)", ""),
        ("j / k", "Scroll summary text"),
        ("Esc", "Close & insert summary into ## Summary section"),
        ("", ""),
        ("Freshness (Shift+F)", ""),
        ("j / k", "Navigate notes by staleness severity"),
        ("Enter", "Open selected note"),
        ("Esc", "Close freshness panel"),
        ("", ""),
        ("Annotations (A)", ""),
        ("n", "New annotation for the open note"),
        ("i", "Mark focused annotation as incorporated"),
        ("d", "Mark focused annotation as ignored"),
        ("j / k", "Navigate annotations"),
        ("Esc", "Close annotation panel"),
        ("", ""),
        ("Kazam KB Browser (B)", ""),
        ("(type)", "Filter pages by title or slug"),
        ("j / k", "Navigate pages"),
        ("Enter", "Import page (or open if already imported)"),
        ("Esc", "Close KB browser"),
        ("", ""),
        ("Freshness frontmatter fields", ""),
        (
            "review_every",
            "Cadence: 7d  2w  3m  1y  monthly  quarterly  yearly",
        ),
        ("owner", "Who is responsible for reviewing this note"),
        ("expires", "Hard expiry date: YYYY-MM-DD"),
        ("sources_of_truth", "List of {label, href} reference links"),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (key, desc) in bindings {
        if desc.is_empty() && !key.is_empty() {
            lines.push(Line::from(Span::styled(
                key.to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
        } else if key.is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<20}", key), Style::default().fg(Color::Yellow)),
                Span::raw(desc.to_string()),
            ]));
        }
    }

    let para = Paragraph::new(Text::from(lines)).scroll((state.help_scroll as u16, 0));
    f.render_widget(para, inner);
}
