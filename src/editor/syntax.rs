use crate::editor::buffer::TextBuffer;
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};
use std::path::Path;
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, Style as SyntectStyle, Theme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};

const WARMUP_LINES: usize = 150;
const MAX_HIGHLIGHT_LINE: usize = 2000;

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let mut themes = ThemeSet::load_defaults().themes;
        let theme = themes
            .remove("base16-ocean.dark")
            .or_else(|| themes.into_values().next())
            .unwrap_or_default();

        Self { syntax_set, theme }
    }

    fn find_syntax(&self, path: Option<&Path>) -> &SyntaxReference {
        let by_extension = path
            .and_then(|p| p.extension())
            .and_then(|ext| ext.to_str())
            .and_then(|ext| self.syntax_set.find_syntax_by_extension(ext));

        let by_name = || {
            path.and_then(|p| p.file_name())
                .and_then(|name| name.to_str())
                .and_then(|name| self.syntax_set.find_syntax_by_extension(name))
        };

        by_extension
            .or_else(by_name)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }

    pub fn highlight_window(
        &self,
        buffer: &TextBuffer,
        start: usize,
        end: usize,
    ) -> Vec<Vec<Span<'static>>> {
        let syntax = self.find_syntax(buffer.file_path.as_deref());
        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut lines = Vec::with_capacity(end.saturating_sub(start));

        for row in start.saturating_sub(WARMUP_LINES)..end {
            let text = buffer.line_text(row);

            if text.len() > MAX_HIGHLIGHT_LINE {
                if row >= start {
                    lines.push(vec![Span::raw(text)]);
                }
                continue;
            }

            let mut source = text.clone();
            source.push('\n');
            let ranges = highlighter.highlight_line(&source, &self.syntax_set);

            if row < start {
                continue;
            }

            match ranges {
                Ok(ranges) => {
                    let spans = ranges
                        .into_iter()
                        .filter_map(|(style, piece)| {
                            let piece = piece.trim_end_matches('\n');
                            if piece.is_empty() {
                                None
                            } else {
                                Some(Span::styled(piece.to_owned(), convert_style(style)))
                            }
                        })
                        .collect();
                    lines.push(spans);
                }
                Err(_) => lines.push(vec![Span::raw(text)]),
            }
        }

        lines
    }
}

fn convert_style(style: SyntectStyle) -> Style {
    let foreground = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
    let mut result = Style::default().fg(foreground);

    if style.font_style.contains(FontStyle::BOLD) {
        result = result.add_modifier(Modifier::BOLD);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        result = result.add_modifier(Modifier::ITALIC);
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        result = result.add_modifier(Modifier::UNDERLINED);
    }

    result
}
