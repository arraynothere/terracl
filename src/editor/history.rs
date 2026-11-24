const LIMIT: usize = 1000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub start: usize,
    pub removed: String,
    pub inserted: String,
    pub before: (usize, usize),
    pub after: (usize, usize),
    pub chain: bool,
}

#[derive(Default)]
pub struct History {
    undo: Vec<Edit>,
    redo: Vec<Edit>,
}

impl History {
    pub fn record(&mut self, edit: Edit) {
        self.redo.clear();

        if let Some(last) = self.undo.last_mut() {
            if merge(last, &edit) {
                return;
            }
        }

        self.undo.push(edit);
        if self.undo.len() > LIMIT {
            let excess = self.undo.len() - LIMIT;
            self.undo.drain(..excess);
        }
    }

    pub fn undo(&mut self) -> Vec<Edit> {
        let mut group = Vec::new();
        while let Some(edit) = self.undo.pop() {
            let chained = edit.chain;
            self.redo.push(edit.clone());
            group.push(edit);
            if !chained {
                break;
            }
        }
        group
    }

    pub fn redo(&mut self) -> Vec<Edit> {
        let mut group = Vec::new();
        while let Some(edit) = self.redo.pop() {
            self.undo.push(edit.clone());
            group.push(edit);
            if !self.redo.last().is_some_and(|next| next.chain) {
                break;
            }
        }
        group
    }
}

fn single_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

fn is_break(c: char) -> bool {
    c == '\n' || c == '\r'
}

fn merge(last: &mut Edit, edit: &Edit) -> bool {
    if last.chain || edit.chain {
        return false;
    }
    if last.removed.is_empty() && edit.removed.is_empty() {
        return merge_typing(last, edit);
    }
    if last.inserted.is_empty() && edit.inserted.is_empty() {
        return merge_deletion(last, edit);
    }
    false
}

fn merge_typing(last: &mut Edit, edit: &Edit) -> bool {
    let Some(typed) = single_char(&edit.inserted) else {
        return false;
    };
    if is_break(typed) || last.inserted.chars().any(is_break) {
        return false;
    }
    if edit.start != last.start + last.inserted.chars().count() {
        return false;
    }
    let ends_with_space = last
        .inserted
        .chars()
        .last()
        .is_some_and(char::is_whitespace);
    if typed.is_whitespace() && !ends_with_space {
        return false;
    }
    last.inserted.push(typed);
    last.after = edit.after;
    true
}

fn merge_deletion(last: &mut Edit, edit: &Edit) -> bool {
    let Some(removed) = single_char(&edit.removed) else {
        return false;
    };
    if is_break(removed) || last.removed.chars().any(is_break) {
        return false;
    }
    if edit.start + 1 == last.start {
        last.removed.insert(0, removed);
        last.start = edit.start;
    } else if edit.start == last.start {
        last.removed.push(removed);
    } else {
        return false;
    }
    last.after = edit.after;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(start: usize, c: char) -> Edit {
        Edit {
            start,
            removed: String::new(),
            inserted: c.to_string(),
            before: (0, start),
            after: (0, start + 1),
            chain: false,
        }
    }

    fn deleted(start: usize, c: char) -> Edit {
        Edit {
            start,
            removed: c.to_string(),
            inserted: String::new(),
            before: (0, start + 1),
            after: (0, start),
            chain: false,
        }
    }

    fn replaced(start: usize, chain: bool) -> Edit {
        Edit {
            start,
            removed: String::from("a"),
            inserted: String::from("b"),
            before: (0, start),
            after: (0, start + 1),
            chain,
        }
    }

    #[test]
    fn typing_run_is_coalesced() {
        let mut history = History::default();
        history.record(typed(0, 'a'));
        history.record(typed(1, 'b'));
        history.record(typed(2, 'c'));
        let group = history.undo();
        assert_eq!(group.len(), 1);
        assert_eq!(group[0].inserted, "abc");
        assert_eq!(group[0].after, (0, 3));
        assert!(history.undo().is_empty());
    }

    #[test]
    fn typing_breaks_at_word_boundary() {
        let mut history = History::default();
        history.record(typed(0, 'a'));
        history.record(typed(1, ' '));
        history.record(typed(2, 'b'));
        assert_eq!(history.undo.len(), 2);
        assert_eq!(history.undo[1].inserted, " b");
    }

    #[test]
    fn typing_breaks_at_newline() {
        let mut history = History::default();
        history.record(typed(0, 'a'));
        history.record(typed(1, '\n'));
        history.record(typed(2, 'b'));
        assert_eq!(history.undo.len(), 3);
    }

    #[test]
    fn non_adjacent_typing_is_not_merged() {
        let mut history = History::default();
        history.record(typed(0, 'a'));
        history.record(typed(5, 'b'));
        assert_eq!(history.undo.len(), 2);
    }

    #[test]
    fn backspace_run_is_coalesced() {
        let mut history = History::default();
        history.record(deleted(2, 'c'));
        history.record(deleted(1, 'b'));
        history.record(deleted(0, 'a'));
        assert_eq!(history.undo.len(), 1);
        assert_eq!(history.undo[0].removed, "abc");
        assert_eq!(history.undo[0].start, 0);
    }

    #[test]
    fn forward_delete_run_is_coalesced() {
        let mut history = History::default();
        history.record(deleted(1, 'b'));
        history.record(deleted(1, 'c'));
        assert_eq!(history.undo.len(), 1);
        assert_eq!(history.undo[0].removed, "bc");
        assert_eq!(history.undo[0].start, 1);
    }

    #[test]
    fn redo_returns_what_undo_removed() {
        let mut history = History::default();
        history.record(typed(0, 'a'));
        let undone = history.undo();
        let redone = history.redo();
        assert_eq!(undone, redone);
        assert!(history.redo().is_empty());
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut history = History::default();
        history.record(typed(0, 'a'));
        history.undo();
        history.record(typed(0, 'b'));
        assert!(history.redo().is_empty());
    }

    #[test]
    fn chained_edits_undo_and_redo_together() {
        let mut history = History::default();
        history.record(replaced(0, false));
        history.record(replaced(10, true));
        history.record(replaced(20, true));
        let undone = history.undo();
        assert_eq!(undone.len(), 3);
        assert_eq!(undone[0].start, 20);
        assert_eq!(undone[2].start, 0);
        let redone = history.redo();
        assert_eq!(redone.len(), 3);
        assert_eq!(redone[0].start, 0);
        assert_eq!(redone[2].start, 20);
    }

    #[test]
    fn separate_groups_stay_separate() {
        let mut history = History::default();
        history.record(replaced(0, false));
        history.record(replaced(10, true));
        history.record(replaced(30, false));
        assert_eq!(history.undo().len(), 1);
        assert_eq!(history.undo().len(), 2);
    }

    #[test]
    fn history_is_bounded() {
        let mut history = History::default();
        for index in 0..LIMIT + 50 {
            history.record(replaced(index * 10, false));
        }
        assert_eq!(history.undo.len(), LIMIT);
    }
}
