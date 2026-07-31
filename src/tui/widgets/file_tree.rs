use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

use crate::{
    app::{AppState, TreeGroupBy, TreeItem},
    notes::FileNode,
};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .tree_display
        .iter()
        .map(|item| match item {
            TreeItem::Header { label, depth } => make_header(label, *depth),
            TreeItem::Node(idx) => make_item(&state.file_tree[*idx]),
        })
        .collect();

    let group_suffix = match state.tree_group_by {
        TreeGroupBy::None => "",
        TreeGroupBy::ModifiedDate => " [modified]",
        TreeGroupBy::CreatedDate => " [created]",
    };

    let block = Block::default()
        .title(format!(" Notes{group_suffix} "))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_file_idx));

    f.render_stateful_widget(list, area, &mut list_state);
}

fn make_header(label: &str, depth: usize) -> ListItem<'static> {
    let indent = "  ".repeat(depth);
    let line = Line::from(Span::styled(
        format!("{indent}  ── {label} "),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    ));
    ListItem::new(line)
}

fn make_item(node: &FileNode) -> ListItem<'static> {
    let indent = "  ".repeat(node.depth);
    let icon = if node.is_dir {
        if node.expanded {
            "▼ "
        } else {
            "▶ "
        }
    } else {
        "  "
    };

    let name_style = if node.is_dir {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let line = Line::from(vec![
        Span::raw(format!("{indent}{icon}")),
        Span::styled(node.name.clone(), name_style),
    ]);

    ListItem::new(line)
}
