use crate::{
    editor::{
        history::{Edit, History},
        text::char_width,
    },
    system::io::LoadedFile,
};
use encoding_rs::{Encoding, UTF_8};
use ropey::{Rope, RopeSlice};
use std::path::PathBuf;

pub struct TextBuffer {
    pub rope: Rope,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_row: usize,
    pub scroll_col: usize,
    pub file_path: Option<PathBuf>,
    pub encoding: &'static Encoding,
    pub bom: bool,
    pub line_ending: &'static str,
    pub modified: bool,
    anchor: Option<(usize, usize)>,
    history: History,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

fn trimmed_len(line: RopeSlice) -> usize {
    let len = line.len_chars();
    if len == 0 {
        return 0;
    }
    match line.char(len - 1) {
        '\n' => {
            if len > 1 && line.char(len - 2) == '\r' {
                len - 2
            } else {
                len - 1
            }
        }
        '\r' | '\u{0B}' | '\u{0C}' | '\u{85}' | '\u{2028}' | '\u{2029}' => len - 1,
        _ => len,
    }
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            file_path: None,
            encoding: UTF_8,
            bom: false,
            line_ending: "\n",
            modified: false,
            anchor: None,
            history: History::default(),
        }
    }

    pub fn from_loaded(file: LoadedFile, file_path: Option<PathBuf>) -> Self {
        Self {
            rope: file.rope,
            file_path,
            encoding: file.encoding,
            bom: file.bom,
            line_ending: file.line_ending,
            ..Self::new()
        }
    }

    pub fn display_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from("[New File]"))
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line_len(&self, row: usize) -> usize {
        if row >= self.rope.len_lines() {
            return 0;
        }
        trimmed_len(self.rope.line(row))
    }

    pub fn line_text(&self, row: usize) -> String {
        if row >= self.rope.len_lines() {
            return String::new();
        }
        let line = self.rope.line(row);
        line.slice(..trimmed_len(line)).to_string()
    }

    fn char_index(&self, row: usize, col: usize) -> usize {
        self.rope.line_to_char(row) + col.min(self.line_len(row))
    }

    fn cursor_index(&self) -> usize {
        self.char_index(self.cursor_row, self.cursor_col)
    }

    fn set_cursor(&mut self, position: (usize, usize)) {
        let row = position.0.min(self.rope.len_lines() - 1);
        self.cursor_row = row;
        self.cursor_col = position.1.min(self.line_len(row));
    }

    fn set_cursor_from_index(&mut self, index: usize) {
        let index = index.min(self.rope.len_chars());
        let row = self.rope.char_to_line(index);
        let col = index - self.rope.line_to_char(row);
        self.cursor_row = row;
        self.cursor_col = col.min(self.line_len(row));
    }

    fn width_before_cursor(&self) -> usize {
        let chars: Vec<char> = self.line_text(self.cursor_row).chars().collect();
        let end = self.cursor_col.min(chars.len());
        let start = self.scroll_col.min(end);
        chars[start..end].iter().map(|&c| char_width(c)).sum()
    }

    pub fn cursor_offset(&self) -> usize {
        self.width_before_cursor()
    }

    pub fn has_anchor(&self) -> bool {
        self.anchor.is_some()
    }

    pub fn begin_selection(&mut self) {
        if self.anchor.is_none() {
            self.anchor = Some((self.cursor_row, self.cursor_col));
        }
    }

    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    pub fn select_all(&mut self) {
        self.anchor = Some((0, 0));
        self.set_cursor_from_index(self.rope.len_chars());
    }

    fn selection_span(&self) -> Option<(usize, usize)> {
        let (row, col) = self.anchor?;
        let start = self.char_index(row.min(self.rope.len_lines() - 1), col);
        let end = self.cursor_index();
        if start == end {
            None
        } else {
            Some((start.min(end), start.max(end)))
        }
    }

    pub fn selection_columns(&self, row: usize) -> Option<(usize, usize)> {
        let (start, end) = self.selection_span()?;
        let first = self.rope.char_to_line(start);
        let last = self.rope.char_to_line(end);
        if row < first || row > last {
            return None;
        }
        let line_start = self.rope.line_to_char(row);
        let from = if row == first { start - line_start } else { 0 };
        let to = if row == last {
            end - line_start
        } else {
            self.line_len(row)
        };
        if from < to {
            Some((from, to))
        } else {
            None
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection_span()?;
        Some(self.rope.slice(start..end).to_string())
    }

    pub fn delete_selection(&mut self) -> bool {
        match self.selection_span() {
            Some((start, end)) => {
                self.edit(start, end, "", false);
                true
            }
            None => {
                self.anchor = None;
                false
            }
        }
    }

    fn edit_range(&self) -> (usize, usize) {
        self.selection_span().unwrap_or_else(|| {
            let index = self.cursor_index();
            (index, index)
        })
    }

    fn edit(&mut self, start: usize, end: usize, text: &str, chain: bool) {
        let removed = self.rope.slice(start..end).to_string();
        if removed.is_empty() && text.is_empty() {
            return;
        }

        let before = (self.cursor_row, self.cursor_col);
        if end > start {
            self.rope.remove(start..end);
        }
        if !text.is_empty() {
            self.rope.insert(start, text);
        }

        self.set_cursor_from_index(start + text.chars().count());
        self.anchor = None;
        self.modified = true;
        self.history.record(Edit {
            start,
            removed,
            inserted: text.to_owned(),
            before,
            after: (self.cursor_row, self.cursor_col),
            chain,
        });
    }

    pub fn insert_str(&mut self, text: &str) {
        let (start, end) = self.edit_range();
        self.edit(start, end, text, false);
    }

    pub fn insert_char(&mut self, ch: char) {
        let mut buffer = [0u8; 4];
        self.insert_str(ch.encode_utf8(&mut buffer));
    }

    pub fn insert_text(&mut self, text: &str) {
        let normalized = text
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace('\n', self.line_ending);
        self.insert_str(&normalized);
    }

    pub fn insert_newline(&mut self) {
        let indent: String = self
            .line_text(self.cursor_row)
            .chars()
            .take(self.cursor_col)
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let mut text = String::from(self.line_ending);
        text.push_str(&indent);
        self.insert_str(&text);
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }

        let index = self.cursor_index();
        if self.cursor_col > 0 {
            self.edit(index - 1, index, "", false);
        } else if self.cursor_row > 0 {
            let previous = self.cursor_row - 1;
            let previous_end = self.rope.line_to_char(previous) + self.line_len(previous);
            let line_start = self.rope.line_to_char(self.cursor_row);
            self.edit(previous_end, line_start, "", false);
        }
    }

    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }

        let index = self.cursor_index();
        if self.cursor_col < self.line_len(self.cursor_row) {
            self.edit(index, index + 1, "", false);
        } else if self.cursor_row + 1 < self.rope.len_lines() {
            let next_start = self.rope.line_to_char(self.cursor_row + 1);
            self.edit(index, next_start, "", false);
        }
    }

    pub fn undo(&mut self) -> bool {
        let edits = self.history.undo();
        if edits.is_empty() {
            return false;
        }
        for edit in &edits {
            self.revert(edit);
        }
        self.finish_history_step();
        true
    }

    pub fn redo(&mut self) -> bool {
        let edits = self.history.redo();
        if edits.is_empty() {
            return false;
        }
        for edit in &edits {
            self.reapply(edit);
        }
        self.finish_history_step();
        true
    }

    fn revert(&mut self, edit: &Edit) {
        let end = edit.start + edit.inserted.chars().count();
        if end > edit.start {
            self.rope.remove(edit.start..end);
        }
        if !edit.removed.is_empty() {
            self.rope.insert(edit.start, &edit.removed);
        }
        self.set_cursor(edit.before);
    }

    fn reapply(&mut self, edit: &Edit) {
        let end = edit.start + edit.removed.chars().count();
        if end > edit.start {
            self.rope.remove(edit.start..end);
        }
        if !edit.inserted.is_empty() {
            self.rope.insert(edit.start, &edit.inserted);
        }
        self.set_cursor(edit.after);
    }

    fn finish_history_step(&mut self) {
        self.anchor = None;
        self.modified = true;
    }

    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.cursor_col.min(self.line_len(self.cursor_row));
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.rope.len_lines() {
            self.cursor_row += 1;
            self.cursor_col = self.cursor_col.min(self.line_len(self.cursor_row));
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.line_len(self.cursor_row);
        }
    }

    pub fn move_right(&mut self) {
        let max_col = self.line_len(self.cursor_row);
        if self.cursor_col < max_col {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.rope.len_lines() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_col = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_col = self.line_len(self.cursor_row);
    }

    pub fn page_up(&mut self, step: usize) {
        self.cursor_row = self.cursor_row.saturating_sub(step);
        self.cursor_col = self.cursor_col.min(self.line_len(self.cursor_row));
    }

    pub fn page_down(&mut self, step: usize) {
        let max_row = self.rope.len_lines().saturating_sub(1);
        self.cursor_row = (self.cursor_row + step).min(max_row);
        self.cursor_col = self.cursor_col.min(self.line_len(self.cursor_row));
    }

    pub fn update_scroll(&mut self, viewport_height: usize, viewport_width: usize) {
        if viewport_height > 0 {
            if self.cursor_row < self.scroll_row {
                self.scroll_row = self.cursor_row;
            } else if self.cursor_row >= self.scroll_row + viewport_height {
                self.scroll_row = self.cursor_row + 1 - viewport_height;
            }
        }

        if viewport_width == 0 {
            return;
        }

        if self.cursor_col < self.scroll_col {
            self.scroll_col = self.cursor_col;
            return;
        }

        let chars: Vec<char> = self.line_text(self.cursor_row).chars().collect();
        let end = self.cursor_col.min(chars.len());
        let mut scroll = self.scroll_col.min(end);
        let mut offset: usize = chars[scroll..end].iter().map(|&c| char_width(c)).sum();
        while offset >= viewport_width && scroll < end {
            offset -= char_width(chars[scroll]);
            scroll += 1;
        }
        self.scroll_col = scroll;
    }

    pub fn find_next(&mut self, query: &str) -> bool {
        if query.is_empty() {
            return false;
        }

        let total = self.line_count();
        for step in 0..=total {
            let row = (self.cursor_row + step) % total;
            let text = self.line_text(row);
            let from_col = if step == 0 { self.cursor_col + 1 } else { 0 };
            let Some((from_byte, _)) = text.char_indices().nth(from_col) else {
                continue;
            };
            if let Some(offset) = text[from_byte..].find(query) {
                self.cursor_row = row;
                self.cursor_col = text[..from_byte + offset].chars().count();
                return true;
            }
        }
        false
    }

    pub fn replace_all(&mut self, query: &str, replacement: &str) -> usize {
        if query.is_empty() {
            return 0;
        }

        let origin = (self.cursor_row, self.cursor_col);
        let mut count = 0;
        let mut chained = false;

        for row in 0..self.line_count() {
            let text = self.line_text(row);
            let matches = text.matches(query).count();
            if matches == 0 {
                continue;
            }
            let start = self.rope.line_to_char(row);
            let end = start + text.chars().count();
            self.edit(start, end, &text.replace(query, replacement), chained);
            chained = true;
            count += matches;
        }

        if count > 0 {
            self.set_cursor(origin);
            self.anchor = None;
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer(text: &str) -> TextBuffer {
        let mut buffer = TextBuffer::new();
        buffer.rope = Rope::from_str(text);
        buffer
    }

    fn buffer_at(text: &str, row: usize, col: usize) -> TextBuffer {
        let mut buffer = buffer(text);
        buffer.cursor_row = row;
        buffer.cursor_col = col;
        buffer
    }

    fn content(buffer: &TextBuffer) -> String {
        buffer.rope.to_string()
    }

    #[test]
    fn line_len_ignores_terminators() {
        let buffer = buffer("ab\r\ncd\n");
        assert_eq!(buffer.line_count(), 3);
        assert_eq!(buffer.line_len(0), 2);
        assert_eq!(buffer.line_len(1), 2);
        assert_eq!(buffer.line_len(2), 0);
        assert_eq!(buffer.line_text(0), "ab");
    }

    #[test]
    fn insert_char_moves_cursor() {
        let mut buffer = TextBuffer::new();
        buffer.insert_char('a');
        buffer.insert_char('b');
        assert_eq!(content(&buffer), "ab");
        assert_eq!(buffer.cursor_col, 2);
        assert!(buffer.modified);
    }

    #[test]
    fn newline_copies_indentation() {
        let mut buffer = buffer_at("    foo", 0, 7);
        buffer.insert_newline();
        assert_eq!(content(&buffer), "    foo\n    ");
        assert_eq!((buffer.cursor_row, buffer.cursor_col), (1, 4));
    }

    #[test]
    fn newline_follows_file_line_ending() {
        let mut buffer = buffer_at("ab", 0, 1);
        buffer.line_ending = "\r\n";
        buffer.insert_newline();
        assert_eq!(content(&buffer), "a\r\nb");
        assert_eq!((buffer.cursor_row, buffer.cursor_col), (1, 0));
    }

    #[test]
    fn backspace_joins_lines() {
        let mut buffer = buffer_at("ab\ncd", 1, 0);
        buffer.backspace();
        assert_eq!(content(&buffer), "abcd");
        assert_eq!((buffer.cursor_row, buffer.cursor_col), (0, 2));
    }

    #[test]
    fn backspace_joins_crlf_lines() {
        let mut buffer = buffer_at("ab\r\ncd", 1, 0);
        buffer.backspace();
        assert_eq!(content(&buffer), "abcd");
    }

    #[test]
    fn delete_joins_lines() {
        let mut buffer = buffer_at("ab\ncd", 0, 2);
        buffer.delete();
        assert_eq!(content(&buffer), "abcd");
        assert_eq!((buffer.cursor_row, buffer.cursor_col), (0, 2));
    }

    #[test]
    fn movement_wraps_between_lines() {
        let mut buffer = buffer_at("ab\ncd", 0, 2);
        buffer.move_right();
        assert_eq!((buffer.cursor_row, buffer.cursor_col), (1, 0));
        buffer.move_left();
        assert_eq!((buffer.cursor_row, buffer.cursor_col), (0, 2));
    }

    #[test]
    fn page_down_is_clamped() {
        let mut buffer = buffer("a\nb\nc");
        buffer.page_down(10);
        assert_eq!(buffer.cursor_row, 2);
        buffer.page_up(10);
        assert_eq!(buffer.cursor_row, 0);
    }

    #[test]
    fn undo_and_redo_typing() {
        let mut buffer = TextBuffer::new();
        for c in "abc".chars() {
            buffer.insert_char(c);
        }
        assert!(buffer.undo());
        assert_eq!(content(&buffer), "");
        assert_eq!(buffer.cursor_col, 0);
        assert!(!buffer.undo());
        assert!(buffer.redo());
        assert_eq!(content(&buffer), "abc");
        assert_eq!(buffer.cursor_col, 3);
        assert!(!buffer.redo());
    }

    #[test]
    fn undo_backspace_restores_text_and_cursor() {
        let mut buffer = buffer_at("abc", 0, 3);
        buffer.backspace();
        assert_eq!(content(&buffer), "ab");
        assert!(buffer.undo());
        assert_eq!(content(&buffer), "abc");
        assert_eq!(buffer.cursor_col, 3);
    }

    #[test]
    fn undo_newline_restores_text() {
        let mut buffer = buffer_at("  ab", 0, 4);
        buffer.insert_newline();
        assert!(buffer.undo());
        assert_eq!(content(&buffer), "  ab");
        assert_eq!((buffer.cursor_row, buffer.cursor_col), (0, 4));
    }

    #[test]
    fn edit_after_undo_discards_redo() {
        let mut buffer = TextBuffer::new();
        buffer.insert_char('a');
        buffer.undo();
        buffer.insert_char('b');
        assert!(!buffer.redo());
        assert_eq!(content(&buffer), "b");
    }

    #[test]
    fn find_next_wraps_around() {
        let mut buffer = buffer("one two\nthree one");
        assert!(buffer.find_next("one"));
        assert_eq!((buffer.cursor_row, buffer.cursor_col), (1, 6));
        assert!(buffer.find_next("one"));
        assert_eq!((buffer.cursor_row, buffer.cursor_col), (0, 0));
    }

    #[test]
    fn find_next_reports_missing_text() {
        let mut buffer = buffer("one two");
        assert!(!buffer.find_next("zzz"));
        assert!(!buffer.find_next(""));
    }

    #[test]
    fn replace_all_is_a_single_undo_step() {
        let original = "foo\nbar foo\nfoo";
        let mut buffer = buffer(original);
        assert_eq!(buffer.replace_all("foo", "x"), 3);
        assert_eq!(content(&buffer), "x\nbar x\nx");
        assert!(buffer.undo());
        assert_eq!(content(&buffer), original);
        assert!(!buffer.undo());
        assert!(buffer.redo());
        assert_eq!(content(&buffer), "x\nbar x\nx");
    }

    #[test]
    fn replace_all_without_match_changes_nothing() {
        let mut buffer = buffer("abc");
        assert_eq!(buffer.replace_all("zzz", "x"), 0);
        assert!(!buffer.modified);
    }

    #[test]
    fn selection_text_and_delete() {
        let mut buffer = buffer("hello world");
        buffer.begin_selection();
        for _ in 0..5 {
            buffer.move_right();
        }
        assert_eq!(buffer.selected_text().as_deref(), Some("hello"));
        assert!(buffer.delete_selection());
        assert_eq!(content(&buffer), " world");
        assert!(buffer.undo());
        assert_eq!(content(&buffer), "hello world");
    }

    #[test]
    fn typing_replaces_selection() {
        let mut buffer = buffer("hello world");
        buffer.begin_selection();
        for _ in 0..5 {
            buffer.move_right();
        }
        buffer.insert_char('X');
        assert_eq!(content(&buffer), "X world");
        assert_eq!(buffer.cursor_col, 1);
        assert!(!buffer.has_anchor());
    }

    #[test]
    fn empty_selection_is_ignored() {
        let mut buffer = buffer("abc");
        buffer.begin_selection();
        assert!(buffer.selected_text().is_none());
        assert!(!buffer.delete_selection());
    }

    #[test]
    fn select_all_covers_buffer() {
        let mut buffer = buffer("ab\ncd");
        buffer.select_all();
        assert_eq!(buffer.selected_text().as_deref(), Some("ab\ncd"));
        assert_eq!((buffer.cursor_row, buffer.cursor_col), (1, 2));
    }

    #[test]
    fn selection_columns_cover_each_row() {
        let mut buffer = buffer("abcd\nefgh\nijkl");
        buffer.cursor_col = 2;
        buffer.begin_selection();
        buffer.cursor_row = 2;
        buffer.cursor_col = 1;
        assert_eq!(buffer.selection_columns(0), Some((2, 4)));
        assert_eq!(buffer.selection_columns(1), Some((0, 4)));
        assert_eq!(buffer.selection_columns(2), Some((0, 1)));
        assert_eq!(buffer.selection_columns(3), None);
    }

    #[test]
    fn paste_normalizes_line_endings() {
        let mut buffer = TextBuffer::new();
        buffer.insert_text("a\r\nb\rc");
        assert_eq!(content(&buffer), "a\nb\nc");

        let mut crlf = TextBuffer::new();
        crlf.line_ending = "\r\n";
        crlf.insert_text("a\nb");
        assert_eq!(content(&crlf), "a\r\nb");
    }

    #[test]
    fn paste_is_one_undo_step() {
        let mut buffer = TextBuffer::new();
        buffer.insert_text("one\ntwo");
        assert!(buffer.undo());
        assert_eq!(content(&buffer), "");
    }

    #[test]
    fn cursor_offset_counts_wide_characters() {
        let buffer = buffer_at("日本語x", 0, 2);
        assert_eq!(buffer.cursor_offset(), 4);
    }

    #[test]
    fn horizontal_scroll_keeps_cursor_visible() {
        let mut buffer = buffer_at("0123456789", 0, 10);
        buffer.update_scroll(10, 5);
        assert_eq!(buffer.scroll_col, 6);
        assert_eq!(buffer.cursor_offset(), 4);
    }

    #[test]
    fn vertical_scroll_keeps_cursor_visible() {
        let mut buffer = buffer_at(&"x\n".repeat(50), 20, 0);
        buffer.update_scroll(10, 40);
        assert_eq!(buffer.scroll_row, 11);
    }
}
