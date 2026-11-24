use crate::{
    app::App,
    editor::text::{fit, str_width},
};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

const SHORTCUTS: [(&str, &str); 15] = [
    ("^S", "Save"),
    ("^Q", "Quit"),
    ("^F", "Find"),
    ("^Z", "Undo"),
    ("^Y", "Redo"),
    ("^C", "Copy"),
    ("^X", "Cut"),
    ("^V", "Paste"),
    ("^R", "Replace"),
    ("^E", "Encoding"),
    ("^B", "Tree"),
    ("^A", "All"),
    ("^G", "Mark"),
    ("^W", "Focus"),
    ("F3", "Next"),
];

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width as usize;
    let bar = Color::Rgb(30, 33, 40);

    let flag = if app.buffer.modified { "*" } else { "" };
    let left = fit(
        &format!(" {}{} ", app.buffer.display_name(), flag),
        width / 2,
    );

    let position = if width >= 80 {
        format!(
            "Ln {}, Col {}",
            app.buffer.cursor_row + 1,
            app.buffer.cursor_col + 1
        )
    } else {
        format!(
            "{}:{}",
            app.buffer.cursor_row + 1,
            app.buffer.cursor_col + 1
        )
    };
    let right = format!(" {} {} ", position, app.buffer.encoding.name());

    let available = width.saturating_sub(str_width(&left) + str_width(&right));

    let key_style = Style::default()
        .bg(Color::White)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().bg(bar).fg(Color::Gray);

    let mut middle: Vec<Span> = Vec::new();
    let mut used = 0;

    if app.status_message.is_empty() {
        for (key, label) in SHORTCUTS {
            let item = str_width(key) + str_width(label) + 2;
            if used + item > available {
                break;
            }
            middle.push(Span::styled(key, key_style));
            middle.push(Span::styled(format!(" {} ", label), label_style));
            used += item;
        }
    } else {
        let message = fit(&format!(" {}", app.status_message), available);
        used = str_width(&message);
        middle.push(Span::styled(
            message,
            Style::default().bg(bar).fg(Color::Yellow),
        ));
    }

    let mut spans = vec![Span::styled(
        left,
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )];
    spans.extend(middle);
    spans.push(Span::styled(
        " ".repeat(available.saturating_sub(used)),
        Style::default().bg(bar),
    ));
    spans.push(Span::styled(
        right,
        Style::default().bg(bar).fg(Color::Cyan),
    ));

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
