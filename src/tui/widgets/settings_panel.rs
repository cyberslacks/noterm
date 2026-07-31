use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::search_overlay::centered_rect;
use crate::app::{AppState, SettingsMode};
use crate::config::{EmbedProvider, LlmProvider};

/// Max chars shown for the system prompt preview in the list row.
const PROMPT_PREVIEW_LEN: usize = 55;

// ── Focusable field indices ───────────────────────────────────────────────────

pub const FIELD_CHAT_PROVIDER: usize = 0;
pub const FIELD_OLLAMA_URL: usize = 1;
pub const FIELD_OLLAMA_CHAT_MODEL: usize = 2;
pub const FIELD_OPENAI_URL: usize = 3;
pub const FIELD_OPENAI_API_KEY: usize = 4;
pub const FIELD_OPENAI_CHAT_MODEL: usize = 5;
pub const FIELD_CLAUDE_MODEL: usize = 6;
pub const FIELD_CLAUDE_API_KEY: usize = 7;
pub const FIELD_EMBED_PROVIDER: usize = 8;
pub const FIELD_OLLAMA_EMBED_MODEL: usize = 9;
pub const FIELD_OPENAI_EMBED_MODEL: usize = 10;
pub const FIELD_SUMMARIZER_URL: usize = 11;
pub const FIELD_SUMMARIZER_API_KEY: usize = 12;
pub const FIELD_SUMMARIZER_MODEL: usize = 13;
pub const FIELD_SUMMARIZER_PROMPT: usize = 14;
pub const FIELD_GIT_USERNAME: usize = 15;
pub const FIELD_GIT_TOKEN: usize = 16;
pub const TOTAL_FIELDS: usize = 17;

pub fn is_model_field(field: usize) -> bool {
    matches!(
        field,
        FIELD_OLLAMA_CHAT_MODEL
            | FIELD_OPENAI_CHAT_MODEL
            | FIELD_OLLAMA_EMBED_MODEL
            | FIELD_OPENAI_EMBED_MODEL
            | FIELD_SUMMARIZER_MODEL
    )
}

pub fn is_provider_field(field: usize) -> bool {
    matches!(field, FIELD_CHAT_PROVIDER | FIELD_EMBED_PROVIDER)
}

pub fn uses_ollama_models(field: usize) -> bool {
    // Summarizer always points at an OpenAI-compat endpoint (OpenWebUI).
    matches!(field, FIELD_OLLAMA_CHAT_MODEL | FIELD_OLLAMA_EMBED_MODEL)
}

/// Get the raw (unmasked) current value of a field from app state.
pub fn get_field_raw(state: &AppState, field: usize) -> String {
    match field {
        FIELD_CHAT_PROVIDER => state.config.llm.provider.to_string(),
        FIELD_OLLAMA_URL => state.config.llm.ollama_base_url.clone(),
        FIELD_OLLAMA_CHAT_MODEL => state.config.llm.ollama_chat_model.clone(),
        FIELD_OPENAI_URL => state.config.llm.openai_base_url.clone(),
        FIELD_OPENAI_API_KEY => state.config.llm.openai_api_key.clone().unwrap_or_default(),
        FIELD_OPENAI_CHAT_MODEL => state.config.llm.openai_model.clone(),
        FIELD_CLAUDE_MODEL => state.config.llm.claude_model.clone(),
        FIELD_CLAUDE_API_KEY => state.config.llm.claude_api_key.clone().unwrap_or_default(),
        FIELD_EMBED_PROVIDER => state.config.llm.embed_provider.to_string(),
        FIELD_OLLAMA_EMBED_MODEL => state.config.llm.ollama_embed_model.clone(),
        FIELD_OPENAI_EMBED_MODEL => state.config.llm.openai_embed_model.clone(),
        FIELD_SUMMARIZER_URL => state.config.summarizer.base_url.clone(),
        FIELD_SUMMARIZER_API_KEY => state.config.summarizer.api_key.clone().unwrap_or_default(),
        FIELD_SUMMARIZER_MODEL => state.config.summarizer.model.clone(),
        FIELD_SUMMARIZER_PROMPT => state.config.summarizer.system_prompt.clone(),
        FIELD_GIT_USERNAME => state.config.git.git_username.clone().unwrap_or_default(),
        FIELD_GIT_TOKEN => state.config.git.git_token.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

/// Write an edited text value back into the config.
pub fn apply_field_value(state: &mut AppState, field: usize, value: String) {
    match field {
        FIELD_OLLAMA_URL => state.config.llm.ollama_base_url = value,
        FIELD_OLLAMA_CHAT_MODEL => state.config.llm.ollama_chat_model = value,
        FIELD_OPENAI_URL => state.config.llm.openai_base_url = value,
        FIELD_OPENAI_API_KEY => {
            state.config.llm.openai_api_key = if value.is_empty() { None } else { Some(value) };
        }
        FIELD_OPENAI_CHAT_MODEL => state.config.llm.openai_model = value,
        FIELD_CLAUDE_MODEL => state.config.llm.claude_model = value,
        FIELD_CLAUDE_API_KEY => {
            state.config.llm.claude_api_key = if value.is_empty() { None } else { Some(value) };
        }
        FIELD_OLLAMA_EMBED_MODEL => state.config.llm.ollama_embed_model = value,
        FIELD_OPENAI_EMBED_MODEL => state.config.llm.openai_embed_model = value,
        FIELD_SUMMARIZER_URL => state.config.summarizer.base_url = value,
        FIELD_SUMMARIZER_API_KEY => {
            state.config.summarizer.api_key = if value.is_empty() { None } else { Some(value) };
        }
        FIELD_SUMMARIZER_MODEL => state.config.summarizer.model = value,
        FIELD_SUMMARIZER_PROMPT => state.config.summarizer.system_prompt = value,
        FIELD_GIT_USERNAME => {
            state.config.git.git_username = if value.is_empty() { None } else { Some(value) };
        }
        FIELD_GIT_TOKEN => {
            state.config.git.git_token = if value.is_empty() { None } else { Some(value) };
        }
        _ => {}
    }
}

pub fn cycle_chat_provider(state: &mut AppState) {
    state.config.llm.provider = match state.config.llm.provider {
        LlmProvider::Ollama => LlmProvider::Claude,
        LlmProvider::Claude => LlmProvider::OpenAI,
        LlmProvider::OpenAI => LlmProvider::Ollama,
    };
}

pub fn cycle_embed_provider(state: &mut AppState) {
    state.config.llm.embed_provider = match state.config.llm.embed_provider {
        EmbedProvider::Ollama => EmbedProvider::OpenAI,
        EmbedProvider::OpenAI => EmbedProvider::Ollama,
    };
}

// ── Rendering ─────────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let popup = centered_rect(85, 85, area);
    f.render_widget(ratatui::widgets::Clear, popup);

    let outer = Block::default()
        .title(
            " Settings  (j/k navigate  Tab/←→ cycle  Enter edit  r re-embed all  Esc save+close) ",
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = outer.inner(popup);
    f.render_widget(outer, popup);

    // Split into left (settings list) and right (model picker / help)
    let picker_visible =
        state.settings_mode == SettingsMode::PickingModel && is_model_field(state.settings_cursor);

    let h_chunks = if picker_visible {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(inner)
    };

    render_settings_list(f, h_chunks[0], state);

    if picker_visible {
        render_model_picker(f, h_chunks[1], state);
    }

    // Prompt editor overlays the settings panel when active.
    if state.settings_mode == SettingsMode::EditingLongText {
        render_prompt_editor(f, area, state);
    }
}

fn render_settings_list(f: &mut Frame, area: Rect, state: &AppState) {
    let cfg = &state.config.llm;
    let scfg = &state.config.summarizer;

    struct Row {
        label: String,
        value: String,
        field_idx: Option<usize>,
        is_header: bool,
    }

    let rows = vec![
        Row {
            label: "── Chat ──────────────────────────".into(),
            value: String::new(),
            field_idx: None,
            is_header: true,
        },
        Row {
            label: "  Chat Provider".into(),
            value: cfg.provider.to_string(),
            field_idx: Some(FIELD_CHAT_PROVIDER),
            is_header: false,
        },
        Row {
            label: "── Ollama ────────────────────────".into(),
            value: String::new(),
            field_idx: None,
            is_header: true,
        },
        Row {
            label: "  URL".into(),
            value: cfg.ollama_base_url.clone(),
            field_idx: Some(FIELD_OLLAMA_URL),
            is_header: false,
        },
        Row {
            label: "  Chat Model".into(),
            value: cfg.ollama_chat_model.clone(),
            field_idx: Some(FIELD_OLLAMA_CHAT_MODEL),
            is_header: false,
        },
        Row {
            label: "── OpenAI / WebUI ────────────────".into(),
            value: String::new(),
            field_idx: None,
            is_header: true,
        },
        Row {
            label: "  URL".into(),
            value: cfg.openai_base_url.clone(),
            field_idx: Some(FIELD_OPENAI_URL),
            is_header: false,
        },
        Row {
            label: "  API Key".into(),
            value: mask_key(&cfg.openai_api_key.clone().unwrap_or_default()),
            field_idx: Some(FIELD_OPENAI_API_KEY),
            is_header: false,
        },
        Row {
            label: "  Chat Model".into(),
            value: cfg.openai_model.clone(),
            field_idx: Some(FIELD_OPENAI_CHAT_MODEL),
            is_header: false,
        },
        Row {
            label: "── Claude ────────────────────────".into(),
            value: String::new(),
            field_idx: None,
            is_header: true,
        },
        Row {
            label: "  Model".into(),
            value: cfg.claude_model.clone(),
            field_idx: Some(FIELD_CLAUDE_MODEL),
            is_header: false,
        },
        Row {
            label: "  API Key".into(),
            value: mask_key(&cfg.claude_api_key.clone().unwrap_or_default()),
            field_idx: Some(FIELD_CLAUDE_API_KEY),
            is_header: false,
        },
        Row {
            label: "── Embeddings ────────────────────".into(),
            value: String::new(),
            field_idx: None,
            is_header: true,
        },
        Row {
            label: "  Embed Provider".into(),
            value: cfg.embed_provider.to_string(),
            field_idx: Some(FIELD_EMBED_PROVIDER),
            is_header: false,
        },
        Row {
            label: "  Ollama Embed Model".into(),
            value: cfg.ollama_embed_model.clone(),
            field_idx: Some(FIELD_OLLAMA_EMBED_MODEL),
            is_header: false,
        },
        Row {
            label: "  OpenAI Embed Model".into(),
            value: cfg.openai_embed_model.clone(),
            field_idx: Some(FIELD_OPENAI_EMBED_MODEL),
            is_header: false,
        },
        Row {
            label: "── Summarizer (X key) ────────────".into(),
            value: String::new(),
            field_idx: None,
            is_header: true,
        },
        Row {
            label: "  URL".into(),
            value: scfg.base_url.clone(),
            field_idx: Some(FIELD_SUMMARIZER_URL),
            is_header: false,
        },
        Row {
            label: "  API Key".into(),
            value: mask_key(&scfg.api_key.clone().unwrap_or_default()),
            field_idx: Some(FIELD_SUMMARIZER_API_KEY),
            is_header: false,
        },
        Row {
            label: "  Model".into(),
            value: scfg.model.clone(),
            field_idx: Some(FIELD_SUMMARIZER_MODEL),
            is_header: false,
        },
        Row {
            label: "  Prompt".into(),
            value: truncate_prompt(&scfg.system_prompt),
            field_idx: Some(FIELD_SUMMARIZER_PROMPT),
            is_header: false,
        },
        Row {
            label: "── Git Credentials (HTTPS) ────────".into(),
            value: String::new(),
            field_idx: None,
            is_header: true,
        },
        Row {
            label: "  Username".into(),
            value: state
                .config
                .git
                .git_username
                .clone()
                .unwrap_or_else(|| "(not set)".into()),
            field_idx: Some(FIELD_GIT_USERNAME),
            is_header: false,
        },
        Row {
            label: "  Token / Password".into(),
            value: mask_key(&state.config.git.git_token.clone().unwrap_or_default()),
            field_idx: Some(FIELD_GIT_TOKEN),
            is_header: false,
        },
    ];

    let items: Vec<ListItem> = rows
        .iter()
        .map(|row| {
            if row.is_header {
                return ListItem::new(Line::from(vec![Span::styled(
                    row.label.clone(),
                    Style::default().fg(Color::DarkGray),
                )]));
            }

            let is_focused = row.field_idx == Some(state.settings_cursor);
            let is_editing = is_focused && state.settings_mode == SettingsMode::EditingText;
            let is_picking = is_focused && state.settings_mode == SettingsMode::PickingModel;

            let cursor = if is_focused { "►" } else { " " };
            let cursor_style = if is_focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let label_style = if is_focused {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };

            let display_value = if is_editing {
                format!("{}█", state.settings_edit_buf)
            } else {
                row.value.clone()
            };

            let value_style = if is_editing {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_picking {
                Style::default().fg(Color::Green)
            } else if is_focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };

            let hint = if is_focused && is_model_field(state.settings_cursor) && !is_picking {
                " [Enter=pick]"
            } else if is_focused && is_provider_field(state.settings_cursor) {
                " [Tab/←→=cycle]"
            } else if is_focused && !is_editing {
                " [Enter=edit]"
            } else {
                ""
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{cursor} "), cursor_style),
                Span::styled(format!("{:<22}", row.label), label_style),
                Span::styled(display_value, value_style),
                Span::styled(hint, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), area);
}

fn render_model_picker(f: &mut Frame, area: Rect, state: &AppState) {
    let models = if uses_ollama_models(state.settings_cursor) {
        &state.available_ollama_models
    } else {
        &state.available_openai_models
    };

    let source = if uses_ollama_models(state.settings_cursor) {
        "Ollama"
    } else {
        "OpenAI/WebUI"
    };

    let block = Block::default()
        .title(format!(" {source} Models "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if models.is_empty() {
        f.render_widget(
            Paragraph::new("(fetching models…)").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = models
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let selected = i == state.settings_model_cursor;
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if selected { "► " } else { "  " };
            ListItem::new(Span::styled(format!("{prefix}{name}"), style))
        })
        .collect();

    f.render_widget(List::new(items), inner);
}

fn render_prompt_editor(f: &mut Frame, area: Rect, state: &AppState) {
    let popup = centered_rect(90, 90, area);
    f.render_widget(ratatui::widgets::Clear, popup);

    let block = Block::default()
        .title(" System Prompt  (edit freely · Esc save+close · Ctrl+s save) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup);
    f.render_widget(block, popup);
    f.render_widget(&state.settings_prompt_editor, inner);
}

fn truncate_prompt(prompt: &str) -> String {
    let first_line = prompt.lines().next().unwrap_or("").trim();
    if first_line.len() > PROMPT_PREVIEW_LEN {
        format!("{}…", &first_line[..PROMPT_PREVIEW_LEN])
    } else {
        format!("{first_line}…")
    }
}

fn mask_key(key: &str) -> String {
    if key.is_empty() {
        "(not set)".into()
    } else if key.len() > 12 {
        format!("{}…{}", &key[..6], &key[key.len() - 4..])
    } else {
        "●●●●●●●●".into()
    }
}
