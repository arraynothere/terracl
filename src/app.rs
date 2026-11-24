use crate::{
    editor::{buffer::TextBuffer, syntax::SyntaxHighlighter},
    system::{clipboard::Clipboard, io::FileHandler, terminal::Tui, tree::FileTree},
    ui::{editor_view, layout},
};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Rect, Size};
use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum FocusArea {
    Sidebar,
    Editor,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum InputMode {
    Normal,
    Find,
    ReplaceSearch,
    ReplaceWith,
    EncodingSelect,
    SaveAs,
    ConfirmUnsaved,
}

#[derive(Clone)]
pub enum PendingAction {
    Quit,
    Open(PathBuf),
}

enum PromptEvent {
    Editing,
    Submitted,
    Cancelled,
}

pub struct App {
    pub buffer: TextBuffer,
    pub highlighter: SyntaxHighlighter,
    pub file_tree: FileTree,
    pub focus: FocusArea,
    pub show_sidebar: bool,
    pub compact: bool,
    pub viewport_height: usize,
    pub input_mode: InputMode,
    pub prompt_input: String,
    pub find_query: String,
    pub replace_query: String,
    pub status_message: String,
    pub encoding_index: usize,
    pub pending: Option<PendingAction>,
    pub clipboard: Clipboard,
    pub marking: bool,
    pub should_quit: bool,
}

fn is_printable(key: &KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    ctrl == alt
}

impl App {
    pub fn new(target: Option<PathBuf>) -> Result<Self> {
        let current_dir = env::current_dir()?;
        let mut root_dir = current_dir.clone();
        let mut buffer = TextBuffer::new();

        if let Some(path) = target {
            if path.is_dir() {
                root_dir = if path.is_relative() {
                    current_dir.join(path)
                } else {
                    path
                };
            } else if path.exists() {
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        root_dir = if parent.is_relative() {
                            current_dir.join(parent)
                        } else {
                            parent.to_path_buf()
                        };
                    }
                }
                let loaded = FileHandler::read_file(&path)?;
                buffer = TextBuffer::from_loaded(loaded, Some(path));
            } else {
                buffer.file_path = Some(path);
            }
        }

        let focus = if buffer.file_path.is_some() {
            FocusArea::Editor
        } else {
            FocusArea::Sidebar
        };

        Ok(Self {
            buffer,
            highlighter: SyntaxHighlighter::new(),
            file_tree: FileTree::new(root_dir),
            focus,
            show_sidebar: true,
            compact: false,
            viewport_height: 1,
            input_mode: InputMode::Normal,
            prompt_input: String::new(),
            find_query: String::new(),
            replace_query: String::new(),
            status_message: String::new(),
            encoding_index: 0,
            pending: None,
            clipboard: Clipboard::default(),
            marking: false,
            should_quit: false,
        })
    }

    pub fn run(&mut self, terminal: &mut Tui) -> Result<()> {
        let size = terminal.size()?;
        self.compact = layout::is_compact(size.width);
        if self.compact {
            self.show_sidebar = self.buffer.file_path.is_none();
        }

        let mut dirty = true;
        while !self.should_quit {
            if dirty {
                self.sync_viewport(terminal.size()?);
                terminal.draw(|frame| layout::render(frame, self))?;
                dirty = false;
            }

            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    self.status_message.clear();
                    self.handle_key_event(key);
                    dirty = true;
                }
                Event::Paste(text) => {
                    self.status_message.clear();
                    self.handle_paste(&text);
                    dirty = true;
                }
                Event::Resize(_, _) => dirty = true,
                _ => {}
            }
        }
        Ok(())
    }

    fn sync_viewport(&mut self, size: Size) {
        self.compact = layout::is_compact(size.width);

        if !self.show_sidebar {
            self.focus = FocusArea::Editor;
        } else if self.compact {
            self.focus = FocusArea::Sidebar;
        }

        let regions = layout::regions(Rect::new(0, 0, size.width, size.height), self.show_sidebar);
        if let Some(area) = regions.editor {
            let view = editor_view::viewport(area, self.buffer.line_count());
            self.viewport_height = view.height.max(1);
            self.buffer.update_scroll(view.height, view.code_width);
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_mode(key),
            InputMode::Find => self.handle_find_mode(&key),
            InputMode::ReplaceSearch => self.handle_replace_search_mode(&key),
            InputMode::ReplaceWith => self.handle_replace_with_mode(&key),
            InputMode::EncodingSelect => self.handle_encoding_mode(&key),
            InputMode::SaveAs => self.handle_save_as_mode(&key),
            InputMode::ConfirmUnsaved => self.handle_confirm_mode(&key),
        }
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT);

        if ctrl {
            if let KeyCode::Char(c) = key.code {
                if self.handle_ctrl(c.to_ascii_lowercase()) {
                    return;
                }
            }
        }

        if key.code == KeyCode::F(3) {
            self.find_again();
            return;
        }

        match self.focus {
            FocusArea::Sidebar => self.handle_sidebar_keys(&key),
            FocusArea::Editor => self.handle_editor_keys(&key),
        }
    }

    fn handle_ctrl(&mut self, c: char) -> bool {
        match c {
            'q' => self.request(PendingAction::Quit),
            's' => self.save(),
            'b' => self.toggle_sidebar(),
            'w' => self.switch_focus(),
            'f' => {
                self.prompt_input = self.find_query.clone();
                self.input_mode = InputMode::Find;
            }
            'r' => {
                self.prompt_input = self.find_query.clone();
                self.input_mode = InputMode::ReplaceSearch;
            }
            'e' => {
                self.encoding_index = FileHandler::supported_encodings()
                    .iter()
                    .position(|&encoding| encoding == self.buffer.encoding)
                    .unwrap_or(0);
                self.input_mode = InputMode::EncodingSelect;
            }
            'z' | 'y' | 'c' | 'x' | 'v' | 'a' | 'g' if self.focus == FocusArea::Editor => {
                self.handle_edit_ctrl(c)
            }
            _ => return false,
        }
        true
    }

    fn handle_edit_ctrl(&mut self, c: char) {
        match c {
            'z' => {
                if !self.buffer.undo() {
                    self.status_message = String::from("Nothing to undo");
                }
            }
            'y' => {
                if !self.buffer.redo() {
                    self.status_message = String::from("Nothing to redo");
                }
            }
            'c' => self.copy_selection(false),
            'x' => self.copy_selection(true),
            'v' => self.paste(),
            'a' => {
                self.buffer.select_all();
                self.marking = false;
            }
            'g' => self.toggle_mark(),
            _ => {}
        }
    }

    fn toggle_mark(&mut self) {
        if self.marking {
            self.marking = false;
            self.buffer.clear_selection();
            self.status_message = String::from("Mark off");
        } else {
            self.marking = true;
            self.buffer.begin_selection();
            self.status_message = String::from("Mark on, move to select");
        }
    }

    fn copy_selection(&mut self, cut: bool) {
        let Some(text) = self.buffer.selected_text() else {
            self.status_message = String::from("Nothing selected");
            return;
        };

        let count = text.chars().count();
        self.clipboard.set(text);
        if cut {
            self.buffer.delete_selection();
            self.marking = false;
        }
        let action = if cut { "Cut" } else { "Copied" };
        self.status_message = format!("{} {} character(s)", action, count);
    }

    fn paste(&mut self) {
        let text = self.clipboard.get().to_owned();
        if text.is_empty() {
            self.status_message = String::from("Clipboard is empty");
            return;
        }
        self.buffer.insert_text(&text);
        self.marking = false;
    }

    fn handle_paste(&mut self, text: &str) {
        match self.input_mode {
            InputMode::Normal => {
                if self.focus == FocusArea::Editor {
                    self.buffer.insert_text(text);
                    self.marking = false;
                }
            }
            InputMode::Find
            | InputMode::ReplaceSearch
            | InputMode::ReplaceWith
            | InputMode::SaveAs => {
                self.prompt_input
                    .push_str(text.lines().next().unwrap_or(""));
            }
            InputMode::EncodingSelect | InputMode::ConfirmUnsaved => {}
        }
    }

    fn toggle_sidebar(&mut self) {
        self.show_sidebar = !self.show_sidebar;
        self.focus = if self.show_sidebar {
            FocusArea::Sidebar
        } else {
            FocusArea::Editor
        };
    }

    fn switch_focus(&mut self) {
        if self.compact {
            self.toggle_sidebar();
        } else if self.show_sidebar {
            self.focus = match self.focus {
                FocusArea::Sidebar => FocusArea::Editor,
                FocusArea::Editor => FocusArea::Sidebar,
            };
        }
    }

    fn handle_sidebar_keys(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Up => self.file_tree.move_up(),
            KeyCode::Down => self.file_tree.move_down(),
            KeyCode::Enter | KeyCode::Right => {
                if let Some(path) = self.file_tree.toggle_or_select() {
                    self.request(PendingAction::Open(path));
                }
            }
            KeyCode::Left => self.file_tree.collapse_current(),
            KeyCode::Esc => {
                self.focus = FocusArea::Editor;
                if self.compact {
                    self.show_sidebar = false;
                }
            }
            _ => {}
        }
    }

    fn handle_editor_keys(&mut self, key: &KeyEvent) {
        let moving = matches!(
            key.code,
            KeyCode::Up
                | KeyCode::Down
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown
        );

        if moving {
            if key.modifiers.contains(KeyModifiers::SHIFT) || self.marking {
                self.buffer.begin_selection();
            } else {
                self.buffer.clear_selection();
            }
        }

        match key.code {
            KeyCode::Up => self.buffer.move_up(),
            KeyCode::Down => self.buffer.move_down(),
            KeyCode::Left => self.buffer.move_left(),
            KeyCode::Right => self.buffer.move_right(),
            KeyCode::Home => self.buffer.move_home(),
            KeyCode::End => self.buffer.move_end(),
            KeyCode::PageUp => self.buffer.page_up(self.viewport_height),
            KeyCode::PageDown => self.buffer.page_down(self.viewport_height),
            KeyCode::Enter => self.buffer.insert_newline(),
            KeyCode::Backspace => self.buffer.backspace(),
            KeyCode::Delete => self.buffer.delete(),
            KeyCode::Tab => self.buffer.insert_str("    "),
            KeyCode::Esc => self.buffer.clear_selection(),
            KeyCode::Char(c) if is_printable(key) => self.buffer.insert_char(c),
            _ => {}
        }

        if !self.buffer.has_anchor() {
            self.marking = false;
        }
    }

    fn edit_prompt(&mut self, key: &KeyEvent) -> PromptEvent {
        match key.code {
            KeyCode::Esc => PromptEvent::Cancelled,
            KeyCode::Enter => PromptEvent::Submitted,
            KeyCode::Backspace => {
                self.prompt_input.pop();
                PromptEvent::Editing
            }
            KeyCode::Char(c) if is_printable(key) => {
                self.prompt_input.push(c);
                PromptEvent::Editing
            }
            _ => PromptEvent::Editing,
        }
    }

    fn handle_find_mode(&mut self, key: &KeyEvent) {
        match self.edit_prompt(key) {
            PromptEvent::Cancelled => self.input_mode = InputMode::Normal,
            PromptEvent::Submitted => {
                self.find_query = self.prompt_input.clone();
                self.input_mode = InputMode::Normal;
                self.find_again();
            }
            PromptEvent::Editing => {}
        }
    }

    fn find_again(&mut self) {
        if self.find_query.is_empty() {
            self.status_message = String::from("Nothing to search for");
        } else if self.buffer.find_next(&self.find_query) {
            self.status_message = format!("Found: {}", self.find_query);
        } else {
            self.status_message = format!("Not found: {}", self.find_query);
        }
    }

    fn handle_replace_search_mode(&mut self, key: &KeyEvent) {
        match self.edit_prompt(key) {
            PromptEvent::Cancelled => self.input_mode = InputMode::Normal,
            PromptEvent::Submitted => {
                self.find_query = self.prompt_input.clone();
                self.prompt_input = self.replace_query.clone();
                self.input_mode = InputMode::ReplaceWith;
            }
            PromptEvent::Editing => {}
        }
    }

    fn handle_replace_with_mode(&mut self, key: &KeyEvent) {
        match self.edit_prompt(key) {
            PromptEvent::Cancelled => self.input_mode = InputMode::Normal,
            PromptEvent::Submitted => {
                self.replace_query = self.prompt_input.clone();
                let count = self
                    .buffer
                    .replace_all(&self.find_query, &self.replace_query);
                self.status_message = format!("Replaced {} occurrence(s)", count);
                self.input_mode = InputMode::Normal;
            }
            PromptEvent::Editing => {}
        }
    }

    fn handle_encoding_mode(&mut self, key: &KeyEvent) {
        let encodings = FileHandler::supported_encodings();
        match key.code {
            KeyCode::Esc => self.input_mode = InputMode::Normal,
            KeyCode::Up => self.encoding_index = self.encoding_index.saturating_sub(1),
            KeyCode::Down => {
                if self.encoding_index + 1 < encodings.len() {
                    self.encoding_index += 1;
                }
            }
            KeyCode::Enter => {
                let selected = encodings[self.encoding_index];
                self.buffer.encoding = selected;
                self.buffer.modified = true;
                self.status_message = format!("Encoding set to {}", selected.name());
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_save_as_mode(&mut self, key: &KeyEvent) {
        match self.edit_prompt(key) {
            PromptEvent::Cancelled => {
                self.input_mode = InputMode::Normal;
                self.pending = None;
            }
            PromptEvent::Submitted => {
                let name = self.prompt_input.trim().to_string();
                if name.is_empty() {
                    return;
                }
                let path = self.file_tree.root.join(name);
                self.input_mode = InputMode::Normal;
                if self.write_to(path) {
                    self.resolve_pending();
                } else {
                    self.pending = None;
                }
            }
            PromptEvent::Editing => {}
        }
    }

    fn handle_confirm_mode(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => match self.buffer.file_path.clone() {
                Some(path) => {
                    self.input_mode = InputMode::Normal;
                    if self.write_to(path) {
                        self.resolve_pending();
                    } else {
                        self.pending = None;
                    }
                }
                None => {
                    self.prompt_input.clear();
                    self.input_mode = InputMode::SaveAs;
                }
            },
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.input_mode = InputMode::Normal;
                self.resolve_pending();
            }
            KeyCode::Esc | KeyCode::Char('c') | KeyCode::Char('C') => {
                self.input_mode = InputMode::Normal;
                self.pending = None;
            }
            _ => {}
        }
    }

    fn request(&mut self, action: PendingAction) {
        if self.buffer.modified {
            self.pending = Some(action);
            self.input_mode = InputMode::ConfirmUnsaved;
        } else {
            self.perform(action);
        }
    }

    fn resolve_pending(&mut self) {
        if let Some(action) = self.pending.take() {
            self.perform(action);
        }
    }

    fn perform(&mut self, action: PendingAction) {
        match action {
            PendingAction::Quit => self.should_quit = true,
            PendingAction::Open(path) => self.open_file(&path),
        }
    }

    fn open_file(&mut self, path: &Path) {
        match FileHandler::read_file(path) {
            Ok(loaded) => {
                self.buffer = TextBuffer::from_loaded(loaded, Some(path.to_path_buf()));
                self.focus = FocusArea::Editor;
                if self.compact {
                    self.show_sidebar = false;
                }
                self.status_message = format!("Opened {}", path.display());
            }
            Err(err) => self.status_message = format!("Open failed: {err:#}"),
        }
    }

    fn save(&mut self) {
        match self.buffer.file_path.clone() {
            Some(path) => {
                self.write_to(path);
            }
            None => {
                self.prompt_input.clear();
                self.input_mode = InputMode::SaveAs;
            }
        }
    }

    fn write_to(&mut self, path: PathBuf) -> bool {
        let result = FileHandler::write_file(
            &path,
            &self.buffer.rope,
            self.buffer.encoding,
            self.buffer.bom,
        );

        match result {
            Ok(()) => {
                self.status_message =
                    format!("Saved {} [{}]", path.display(), self.buffer.encoding.name());
                self.buffer.file_path = Some(path);
                self.buffer.modified = false;
                self.file_tree.refresh();
                true
            }
            Err(err) => {
                self.status_message = format!("Save failed: {err:#}");
                false
            }
        }
    }
}
