use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct LayoutChunks {
    pub title_bar: Rect,
    pub file_tree: Rect,
    pub main_pane: Rect,
    pub chat_pane: Option<Rect>,
    pub status_bar: Rect,
}

pub fn compute(area: Rect, tree_pct: u16, chat_pct: u16, chat_open: bool) -> LayoutChunks {
    // Vertical: title(1) + body(fill) + status(1)
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title_bar = vertical[0];
    let body = vertical[1];
    let status_bar = vertical[2];

    // Horizontal: file tree + main/chat
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(tree_pct),
            Constraint::Percentage(100 - tree_pct),
        ])
        .split(body);

    let file_tree = horizontal[0];
    let right = horizontal[1];

    let (main_pane, chat_pane) = if chat_open {
        let right_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(100 - chat_pct),
                Constraint::Percentage(chat_pct),
            ])
            .split(right);
        (right_split[0], Some(right_split[1]))
    } else {
        (right, None)
    };

    LayoutChunks {
        title_bar,
        file_tree,
        main_pane,
        chat_pane,
        status_bar,
    }
}
