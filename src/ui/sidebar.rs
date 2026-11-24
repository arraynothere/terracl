use crate::app::{App, FocusArea};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == FocusArea::Sidebar;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let root_name = app
        .file_tree
        .root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("EXPLORER"));

    let items: Vec<ListItem> = app
        .file_tree
        .entries
        .iter()
        .map(|entry| {
            let indent = "  ".repeat(entry.depth);
            let marker = if entry.is_dir {
                if entry.is_expanded {
                    "[-] "
                } else {
                    "[+] "
                }
            } else {
                "    "
            };

            let style = if entry.is_dir {
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            ListItem::new(Line::from(vec![
                Span::raw(indent),
                Span::styled(marker, Style::default().fg(Color::DarkGray)),
                Span::styled(&entry.name, style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" {} ", root_name))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 44, 52))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    if !app.file_tree.entries.is_empty() {
        state.select(Some(app.file_tree.selected));
    }

    frame.render_stateful_widget(list, area, &mut state);
}
