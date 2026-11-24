use unicode_width::UnicodeWidthChar;

pub const TAB_WIDTH: usize = 4;

pub fn char_width(c: char) -> usize {
    match c {
        '\t' => TAB_WIDTH,
        c if c.is_control() => 1,
        c => c.width().unwrap_or(0),
    }
}

pub fn str_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

pub fn push_display(out: &mut String, c: char) {
    match c {
        '\t' => out.push_str(&" ".repeat(TAB_WIDTH)),
        c if c.is_control() => out.push('?'),
        c => out.push(c),
    }
}

pub fn fit(text: &str, max_width: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let width = char_width(c);
        if used + width > max_width {
            break;
        }
        used += width;
        out.push(c);
    }
    out
}

pub fn tail(text: &str, max_width: usize) -> String {
    let mut reversed = Vec::new();
    let mut used = 0;
    for c in text.chars().rev() {
        let width = char_width(c);
        if used + width > max_width {
            break;
        }
        used += width;
        reversed.push(c);
    }
    reversed.into_iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_follow_display_cells() {
        assert_eq!(char_width('a'), 1);
        assert_eq!(char_width('日'), 2);
        assert_eq!(char_width('\t'), TAB_WIDTH);
        assert_eq!(char_width('\u{7}'), 1);
        assert_eq!(char_width('\u{0301}'), 0);
        assert_eq!(str_width("a日"), 3);
    }

    #[test]
    fn display_expands_tabs_and_controls() {
        let mut out = String::new();
        push_display(&mut out, '\t');
        push_display(&mut out, '\u{7}');
        push_display(&mut out, 'x');
        assert_eq!(out, "    ?x");
    }

    #[test]
    fn fit_never_splits_wide_characters() {
        assert_eq!(fit("abcdef", 3), "abc");
        assert_eq!(fit("日本語", 5), "日本");
        assert_eq!(fit("abc", 10), "abc");
    }

    #[test]
    fn tail_keeps_the_end_of_the_text() {
        assert_eq!(tail("abcdef", 3), "def");
        assert_eq!(tail("a日", 2), "日");
        assert_eq!(tail("ab", 10), "ab");
    }
}
