use ratatui::{
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use super::search_overlay::centered_rect;
use crate::app::AppState;

pub fn render(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let popup = centered_rect(88, 88, area);
    f.render_widget(ratatui::widgets::Clear, popup);

    let (title, border_color) = if state.summarize_loading {
        (" Generating summary… (Esc to cancel) ", Color::Yellow)
    } else {
        (
            " Summary  [j/k] scroll · [Esc] close & insert into note ",
            Color::Green,
        )
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    if state.summarize_buf.is_empty() && state.summarize_loading {
        let loading =
            Paragraph::new("Connecting to summarizer…").style(Style::default().fg(Color::DarkGray));
        f.render_widget(loading, inner);
    } else {
        let para = Paragraph::new(state.summarize_buf.clone())
            .wrap(Wrap { trim: false })
            .scroll((state.summarize_scroll as u16, 0));
        f.render_widget(para, inner);
    }
}
