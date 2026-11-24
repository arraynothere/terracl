use crate::{
    app::{App, InputMode, PendingAction},
    editor::text::{str_width, tail},
    system::io::FileHandler,
};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    match app.input_mode {
        InputMode::Find => render_input_box(frame, " Find (Enter / Esc) ", &app.prompt_input),
        InputMode::ReplaceSearch => {
            render_input_box(frame, " Replace: search for ", &app.prompt_input)
        }
        InputMode::ReplaceWith => {
            render_input_box(frame, " Replace: replace with ", &app.prompt_input)
        }
        InputMode::SaveAs => render_input_box(frame, " Save as (Enter / Esc) ", &app.prompt_input),
        InputMode::ConfirmUnsaved => render_confirm(frame, app),
        InputMode::EncodingSelect => render_encoding_picker(frame, app),
        InputMode::Normal => {}
    }
}

fn render_input_box(frame: &mut Frame, title: &str, value: &str) {
    let area = centered_rect(60, 32, 3, frame.area());
    frame.render_widget(Clear, area);

    let inner_width = area.width.saturating_sub(2) as usize;
    let visible = tail(value, inner_width.saturating_sub(1));

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let cursor_x = area.x + 1 + str_width(&visible) as u16;
    let text = Paragraph::new(visible)
        .style(Style::default().fg(Color::White))
        .block(block);

    frame.render_widget(text, area);
    frame.set_cursor_position((cursor_x, area.y + 1));
}

fn render_confirm(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 44, 4, frame.area());
    frame.render_widget(Clear, area);

    let question = match &app.pending {
        Some(PendingAction::Open(_)) => "Save changes before opening file?",
        _ => "Save changes before exit?",
    };

    let block = Block::default()
        .title(" Unsaved Changes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let text = Paragraph::new(vec![
        Line::from(question),
        Line::from("[Y]es  [N]o  [C]ancel"),
    ])
    .style(Style::default().fg(Color::White))
    .block(block);

    frame.render_widget(text, area);
}

fn render_encoding_picker(frame: &mut Frame, app: &App) {
    let encodings = FileHandler::supported_encodings();
    let height = (encodings.len() as u16 + 2).min(frame.area().height);
    let area = centered_rect(40, 26, height, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = encodings
        .iter()
        .map(|encoding| ListItem::new(format!(" {} ", encoding.name())))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Select Encoding ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(app.encoding_index));

    frame.render_stateful_widget(list, area, &mut state);
}

fn centered_rect(percent_x: u16, min_width: u16, height: u16, area: Rect) -> Rect {
    let width = ((area.width as u32 * percent_x as u32) / 100) as u16;
    let width = width.max(min_width).min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
