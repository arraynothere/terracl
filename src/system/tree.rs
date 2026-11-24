use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

const IGNORED: [&str; 3] = [".git", "target", "node_modules"];

#[derive(Clone)]
pub struct TreeEntry {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub is_expanded: bool,
}

pub struct FileTree {
    pub root: PathBuf,
    pub entries: Vec<TreeEntry>,
    pub selected: usize,
    expanded_dirs: HashSet<PathBuf>,
}

impl FileTree {
    pub fn new(root: PathBuf) -> Self {
        let mut tree = Self {
            root,
            entries: Vec::new(),
            selected: 0,
            expanded_dirs: HashSet::new(),
        };
        tree.refresh();
        tree
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        let root = self.root.clone();
        self.scan_dir(&root, 0);
        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
    }

    fn scan_dir(&mut self, dir: &Path, depth: usize) {
        let walker = WalkDir::new(dir).min_depth(1).max_depth(1).sort_by(|a, b| {
            b.file_type()
                .is_dir()
                .cmp(&a.file_type().is_dir())
                .then_with(|| {
                    a.file_name()
                        .to_string_lossy()
                        .to_lowercase()
                        .cmp(&b.file_name().to_string_lossy().to_lowercase())
                })
        });

        for item in walker.into_iter().filter_map(|entry| entry.ok()) {
            let name = item.file_name().to_string_lossy().into_owned();
            if IGNORED.contains(&name.as_str()) {
                continue;
            }

            let path = item.path().to_path_buf();
            let is_dir = item.file_type().is_dir();
            let is_expanded = is_dir && self.expanded_dirs.contains(&path);

            self.entries.push(TreeEntry {
                path: path.clone(),
                name,
                depth,
                is_dir,
                is_expanded,
            });

            if is_expanded {
                self.scan_dir(&path, depth + 1);
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    pub fn toggle_or_select(&mut self) -> Option<PathBuf> {
        let entry = self.entries.get(self.selected)?.clone();
        if !entry.is_dir {
            return Some(entry.path);
        }

        if !self.expanded_dirs.remove(&entry.path) {
            self.expanded_dirs.insert(entry.path);
        }
        self.refresh();
        None
    }

    pub fn collapse_current(&mut self) {
        let Some(entry) = self.entries.get(self.selected).cloned() else {
            return;
        };

        if entry.is_dir && entry.is_expanded {
            self.expanded_dirs.remove(&entry.path);
            self.refresh();
            return;
        }

        if let Some(parent) = entry.path.parent() {
            if parent != self.root && self.expanded_dirs.contains(parent) {
                self.expanded_dirs.remove(parent);
                self.refresh();
                if let Some(index) = self.entries.iter().position(|e| e.path == parent) {
                    self.selected = index;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("terracl-tree-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("beta")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("beta").join("inner.rs"), "x").unwrap();
        fs::write(root.join("a.txt"), "x").unwrap();
        fs::write(root.join("Z.txt"), "x").unwrap();
        root
    }

    fn names(tree: &FileTree) -> Vec<&str> {
        tree.entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect()
    }

    #[test]
    fn lists_directories_first_and_skips_ignored() {
        let root = fixture("list");
        let tree = FileTree::new(root.clone());
        let listed = names(&tree).join(",");
        let _ = fs::remove_dir_all(&root);
        assert_eq!(listed, "beta,a.txt,Z.txt");
    }

    #[test]
    fn expanding_a_directory_shows_its_children() {
        let root = fixture("expand");
        let mut tree = FileTree::new(root.clone());
        assert!(tree.toggle_or_select().is_none());
        let listed = names(&tree).join(",");
        let depth = tree.entries[1].depth;
        let _ = fs::remove_dir_all(&root);
        assert_eq!(listed, "beta,inner.rs,a.txt,Z.txt");
        assert_eq!(depth, 1);
    }

    #[test]
    fn selecting_a_file_returns_its_path() {
        let root = fixture("select");
        let mut tree = FileTree::new(root.clone());
        tree.selected = 1;
        let picked = tree.toggle_or_select();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(picked, Some(root.join("a.txt")));
    }

    #[test]
    fn collapse_returns_to_the_parent_directory() {
        let root = fixture("collapse");
        let mut tree = FileTree::new(root.clone());
        tree.toggle_or_select();
        tree.selected = 1;
        tree.collapse_current();
        let listed = names(&tree).join(",");
        let selected = tree.selected;
        let _ = fs::remove_dir_all(&root);
        assert_eq!(listed, "beta,a.txt,Z.txt");
        assert_eq!(selected, 0);
    }

    #[test]
    fn navigation_stays_within_bounds() {
        let root = fixture("bounds");
        let mut tree = FileTree::new(root.clone());
        tree.move_up();
        assert_eq!(tree.selected, 0);
        for _ in 0..10 {
            tree.move_down();
        }
        let last = tree.entries.len() - 1;
        let _ = fs::remove_dir_all(&root);
        assert_eq!(tree.selected, last);
    }
}
