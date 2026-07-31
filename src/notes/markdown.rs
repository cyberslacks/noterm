use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn render(markdown: &str) -> Vec<Line<'static>> {
    let opts = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(markdown, opts);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];
    let mut in_code_block = false;
    let mut code_lang = String::new();

    macro_rules! current_style {
        () => {
            *style_stack.last().unwrap_or(&Style::default())
        };
    }

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
                let (color, modifier) = heading_style(level);
                style_stack.push(Style::default().fg(color).add_modifier(modifier));
            }
            Event::End(TagEnd::Heading(_)) => {
                style_stack.pop();
                lines.push(Line::from(std::mem::take(&mut current_spans)));
                lines.push(Line::from(""));
            }

            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                lines.push(Line::from(std::mem::take(&mut current_spans)));
                lines.push(Line::from(""));
            }

            Event::Start(Tag::Strong) => {
                style_stack.push(current_style!().add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Strong) => {
                style_stack.pop();
            }

            Event::Start(Tag::Emphasis) => {
                style_stack.push(current_style!().add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::Emphasis) => {
                style_stack.pop();
            }

            Event::Start(Tag::Strikethrough) => {
                style_stack.push(current_style!().add_modifier(Modifier::CROSSED_OUT));
            }
            Event::End(TagEnd::Strikethrough) => {
                style_stack.pop();
            }

            Event::Start(Tag::Link { .. }) => {
                style_stack.push(
                    current_style!()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::UNDERLINED),
                );
            }
            Event::End(TagEnd::Link) => {
                style_stack.pop();
            }

            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    _ => String::new(),
                };
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
                let header = if code_lang.is_empty() {
                    "╔═ code ══════════════════".to_string()
                } else {
                    format!("╔═ {} ══════════════════", code_lang)
                };
                lines.push(Line::from(Span::styled(
                    header,
                    Style::default().fg(Color::DarkGray),
                )));
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                lines.push(Line::from(Span::styled(
                    "╚══════════════════════════".to_string(),
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
            }

            Event::Start(Tag::Item) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
                current_spans.push(Span::styled(
                    "  • ".to_string(),
                    Style::default().fg(Color::Yellow),
                ));
            }
            Event::End(TagEnd::Item) => {
                lines.push(Line::from(std::mem::take(&mut current_spans)));
            }

            Event::Start(Tag::BlockQuote(_)) => {
                style_stack.push(
                    current_style!()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                );
            }
            Event::End(TagEnd::BlockQuote) => {
                style_stack.pop();
                lines.push(Line::from(std::mem::take(&mut current_spans)));
            }

            Event::Start(Tag::List(_)) | Event::End(TagEnd::List(_)) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
            }

            Event::Code(code) => {
                current_spans.push(Span::styled(
                    format!(" {code} "),
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray),
                ));
            }

            Event::Text(text) => {
                let style = if in_code_block {
                    Style::default().fg(Color::Green)
                } else {
                    current_style!()
                };

                let prefix = if in_code_block { "  " } else { "" };

                for (i, line_str) in text.lines().enumerate() {
                    if i > 0 {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                    if !line_str.is_empty() {
                        current_spans.push(Span::styled(format!("{prefix}{line_str}"), style));
                    } else if in_code_block {
                        current_spans.push(Span::raw(""));
                    }
                }
            }

            Event::SoftBreak => {
                current_spans.push(Span::raw(" "));
            }

            Event::HardBreak => {
                lines.push(Line::from(std::mem::take(&mut current_spans)));
            }

            Event::Rule => {
                lines.push(Line::from(Span::styled(
                    "─".repeat(60),
                    Style::default().fg(Color::DarkGray),
                )));
            }

            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    lines
}

fn heading_style(level: HeadingLevel) -> (Color, Modifier) {
    match level {
        HeadingLevel::H1 => (Color::Cyan, Modifier::BOLD),
        HeadingLevel::H2 => (Color::Blue, Modifier::BOLD),
        HeadingLevel::H3 => (Color::Green, Modifier::BOLD),
        _ => (Color::Yellow, Modifier::BOLD),
    }
}
