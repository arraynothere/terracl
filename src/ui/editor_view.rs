use crate::{
    app::{App, FocusArea, InputMode},
    editor::text::{char_width, push_display},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

const SELECTION_BG: Color = Color::Rgb(62, 68, 81);

pub struct Viewport {
    pub gutter: u16,
    pub code_width: usize,
    pub height: usize,
}

pub fn viewport(area: Rect, total_lines: usize) -> Viewport {
    let inner = Block::default().borders(Borders::ALL).inner(area);
    let gutter = total_lines.to_string().len().max(3) as u16 + 2;
    Viewport {
        gutter,
        code_width: inner.width.saturating_sub(gutter) as usize,
        height: inner.height as usize,
    }
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == FocusArea::Editor;
    let border_color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let marker = if app.buffer.modified { "* " } else { "" };
    let title = format!(" {} {}", app.buffer.display_name(), marker);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let total_lines = app.buffer.line_count();
    let view = viewport(area, total_lines);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(view.gutter), Constraint::Min(1)])
        .split(inner);
    let gutter_area = columns[0];
    let code_area = columns[1];

    let start_row = app.buffer.scroll_row;
    let end_row = (start_row + view.height).min(total_lines);
    let number_width = (view.gutter - 2) as usize;

    let gutter_lines: Vec<Line> = (start_row..end_row)
        .map(|row| {
            let style = if row == app.buffer.cursor_row {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(Span::styled(
                format!("{:>width$} ", row + 1, width = number_width),
                style,
            ))
        })
        .collect();

    let code_lines: Vec<Line> = app
        .highlighter
        .highlight_window(&app.buffer, start_row, end_row)
        .into_iter()
        .enumerate()
        .map(|(offset, spans)| {
            let selection = app.buffer.selection_columns(start_row + offset);
            Line::from(clip_spans(
                spans,
                app.buffer.scroll_col,
                view.code_width,
                selection,
            ))
        })
        .collect();

    frame.render_widget(Paragraph::new(gutter_lines), gutter_area);
    frame.render_widget(Paragraph::new(code_lines), code_area);

    if focused && app.input_mode == InputMode::Normal {
        let row = app.buffer.cursor_row;
        let offset = app.buffer.cursor_offset();
        if row >= start_row && row < end_row && offset < view.code_width {
            frame.set_cursor_position((
                code_area.x + offset as u16,
                code_area.y + (row - start_row) as u16,
            ));
        }
    }
}

fn clip_spans(
    spans: Vec<Span<'static>>,
    skip: usize,
    max_width: usize,
    selection: Option<(usize, usize)>,
) -> Vec<Span<'static>> {
    let mut clipped = Vec::new();
    let mut column = 0;
    let mut used = 0;

    for span in spans {
        let mut text = String::new();
        let mut current = span.style;

        for c in span.content.chars() {
            let index = column;
            column += 1;
            if index < skip {
                continue;
            }

            let width = char_width(c);
            if used + width > max_width {
                if !text.is_empty() {
                    clipped.push(Span::styled(text, current));
                }
                return clipped;
            }
            used += width;

            let selected = selection.is_some_and(|(from, to)| index >= from && index < to);
            let style = if selected {
                span.style.bg(SELECTION_BG)
            } else {
                span.style
            };

            if style != current {
                if !text.is_empty() {
                    clipped.push(Span::styled(std::mem::take(&mut text), current));
                }
                current = style;
            }
            push_display(&mut text, c);
        }

        if !text.is_empty() {
            clipped.push(Span::styled(text, current));
        }
    }

    clipped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(text: &str) -> Vec<Span<'static>> {
        vec![Span::raw(text.to_owned())]
    }

    fn joined(spans: &[Span]) -> String {
        spans.iter().map(|span| span.content.as_ref()).collect()
    }

    #[test]
    fn clips_to_the_visible_window() {
        let spans = clip_spans(plain("0123456789"), 2, 4, None);
        assert_eq!(joined(&spans), "2345");
    }

    #[test]
    fn expands_tabs_and_counts_wide_characters() {
        let spans = clip_spans(plain("\ta\u{65e5}"), 0, 5, None);
        assert_eq!(joined(&spans), "    a");
        let spans = clip_spans(plain("a\u{65e5}b"), 0, 3, None);
        assert_eq!(joined(&spans), "a\u{65e5}");
    }

    #[test]
    fn keeps_span_styles_when_clipping() {
        let red = Style::default().fg(Color::Red);
        let blue = Style::default().fg(Color::Blue);
        let spans = vec![Span::styled("ab", red), Span::styled("cd", blue)];
        let clipped = clip_spans(spans, 1, 2, None);
        assert_eq!(clipped.len(), 2);
        assert_eq!(clipped[0].style, red);
        assert_eq!(clipped[1].style, blue);
        assert_eq!(joined(&clipped), "bc");
    }

    #[test]
    fn marks_selected_characters() {
        let spans = clip_spans(plain("abcd"), 0, 10, Some((1, 3)));
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1].content, "bc");
        assert_eq!(spans[1].style.bg, Some(SELECTION_BG));
        assert_eq!(spans[0].style.bg, None);
        assert_eq!(spans[2].style.bg, None);
    }

    #[test]
    fn selection_uses_absolute_columns_after_scrolling() {
        let spans = clip_spans(plain("abcdef"), 2, 10, Some((3, 5)));
        assert_eq!(joined(&spans), "cdef");
        assert_eq!(spans[1].content, "de");
        assert_eq!(spans[1].style.bg, Some(SELECTION_BG));
    }
}
