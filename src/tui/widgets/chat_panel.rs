use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::{app::AppState, llm::ChatRole};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let kb_indicator = if state.chat_kazam_context {
        " [KB:ON]"
    } else {
        ""
    };
    let block = Block::default()
        .title(format!(
            " LLM Chat{kb_indicator}  Tab=KB · Ctrl+l=clear · Esc=exit "
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if state.chat_kazam_context {
            Color::Cyan
        } else {
            Color::Blue
        }));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(inner);

    // Message history
    let items: Vec<ListItem> = state
        .chat_messages
        .iter()
        .flat_map(|msg| {
            let (prefix, style) = match msg.role {
                ChatRole::User => (
                    "You: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                ChatRole::Assistant => ("AI:  ", Style::default().fg(Color::Green)),
                ChatRole::System => ("Sys: ", Style::default().fg(Color::DarkGray)),
            };

            // Word-wrap message lines
            let mut lines: Vec<ListItem> = Vec::new();
            for (i, line) in msg.content.lines().enumerate() {
                let p = if i == 0 { prefix } else { "     " };
                lines.push(ListItem::new(Line::from(vec![
                    Span::styled(p.to_string(), style),
                    Span::styled(line.to_string(), Style::default()),
                ])));
            }
            if lines.is_empty() {
                lines.push(ListItem::new(Line::from(Span::styled(
                    prefix.to_string(),
                    style,
                ))));
            }
            lines
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, chunks[0]);

    // Input box
    let loading_indicator = if state.chat_loading { " ⣿" } else { "" };
    let input_block = Block::default()
        .title(format!("Input{loading_indicator}"))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if state.chat_loading {
            Color::Yellow
        } else {
            Color::DarkGray
        }));

    let input = Paragraph::new(state.chat_input.clone())
        .block(input_block)
        .wrap(Wrap { trim: true });
    f.render_widget(input, chunks[1]);
}
