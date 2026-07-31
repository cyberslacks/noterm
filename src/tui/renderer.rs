use ratatui::Frame;

use super::{
    layout,
    widgets::{
        annotation_panel, chat_panel, confirm_delete, editor, file_tree, freshness_panel,
        git_panel, help_popup, kanban, kazam_kb_panel, meetily_panel, search_overlay,
        settings_panel, status_bar, summary_panel, vector_search, viewer,
    },
};
use crate::app::{AppState, Mode};

pub fn render(f: &mut Frame, state: &mut AppState) {
    let area = f.area();
    let chat_open = state.mode == Mode::Chat;

    let layout = layout::compute(
        area,
        state.config.ui.file_tree_width_pct,
        state.config.ui.chat_width_pct,
        chat_open,
    );

    // Title bar
    render_title(f, layout.title_bar, state);

    // Left pane: file tree (always visible)
    file_tree::render(f, layout.file_tree, state);

    // Main pane: depends on mode
    match &state.mode {
        Mode::Kanban => kanban::render(f, layout.main_pane, state),
        Mode::Edit => editor::render(f, layout.main_pane, state),
        _ => viewer::render(f, layout.main_pane, state),
    }

    // Optional chat pane
    if let Some(chat_area) = layout.chat_pane {
        chat_panel::render(f, chat_area, state);
    }

    // Status bar
    status_bar::render(f, layout.status_bar, state);

    // Overlay panels (rendered on top)
    match &state.mode {
        Mode::Search => search_overlay::render(f, area, state),
        Mode::VectorSearch => vector_search::render(f, area, state),
        Mode::Git => git_panel::render(f, area, state),
        Mode::Help => help_popup::render(f, area, state),
        Mode::NewNote | Mode::GitCommitInput => render_prompt_overlay(f, area, state),
        Mode::ConfirmDelete => confirm_delete::render(f, area, state),
        Mode::MeetilyImport => meetily_panel::render(f, area, state),
        Mode::Settings => settings_panel::render(f, area, state),
        Mode::Summarize => summary_panel::render(f, area, state),
        Mode::FreshnessView => freshness_panel::render(f, area, state),
        Mode::AnnotationPanel => annotation_panel::render(f, area, state),
        Mode::KazamKbBrowser => kazam_kb_panel::render(f, area, state),
        _ => {}
    }
}

fn render_title(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    use ratatui::{
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::Paragraph,
    };

    let note_path = state
        .current_note
        .as_ref()
        .map(|n| format!(" │ {}", n.relative_path))
        .unwrap_or_default();

    let branch = state.git_branch();

    let line = Line::from(vec![
        Span::styled(
            " NOTERM ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {}", state.notes_dir.to_string_lossy()),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(note_path, Style::default().fg(Color::White)),
        Span::styled(
            format!("  [git:{branch}]"),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    f.render_widget(Paragraph::new(line), area);
}

fn render_prompt_overlay(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    use super::widgets::search_overlay::centered_rect;
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        style::{Color, Style},
        text::{Line, Span},
        widgets::{Block, BorderType, Borders, Paragraph},
    };

    let popup = centered_rect(50, 15, area);
    f.render_widget(ratatui::widgets::Clear, popup);

    let title = match state.mode {
        Mode::NewNote => " New Note Name (Enter to create, Esc to cancel) ",
        Mode::GitCommitInput => " Commit Message (Enter to commit, Esc to cancel) ",
        _ => " Input ",
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let input_line = Line::from(vec![
        Span::raw("  "),
        Span::raw(state.prompt_input.clone()),
        Span::styled("█", Style::default().fg(Color::Yellow)),
    ]);
    f.render_widget(Paragraph::new(input_line), inner);
}
