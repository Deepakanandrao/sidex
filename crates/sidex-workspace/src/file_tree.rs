//! File tree model — in-memory tree of workspace files.
//!
//! Uses the `ignore` crate to respect `.gitignore` rules and provides
//! lazy loading: only one level deep is scanned initially, with subtrees
//! expanded on demand via [`FileTree::expand`].

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use serde::Serialize;

/// A single node in the file tree — either a file or a directory.
#[derive(Debug, Clone, Serialize)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    /// `None` for files; `Some(vec)` for directories.
    /// An empty `Some(vec![])` means the directory has been expanded but is empty.
    /// `None` on a directory means it has not been expanded yet.
    pub children: Option<Vec<FileNode>>,
}

impl FileNode {
    fn file(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            is_dir: false,
            children: None,
        }
    }

    fn dir(name: String, path: PathBuf, children: Vec<FileNode>) -> Self {
        Self {
            name,
            path,
            is_dir: true,
            children: Some(children),
        }
    }

    fn dir_lazy(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            is_dir: true,
            children: None,
        }
    }
}

/// In-memory file tree for a workspace.
#[derive(Debug, Clone, Serialize)]
pub struct FileTree {
    pub root: FileNode,
}

impl FileTree {
    /// Scan `root` one level deep, respecting `.gitignore`.
    pub fn scan(root: &Path) -> Self {
        let root_name = root
            .file_name()
            .unwrap_or_else(|| OsStr::new(""))
            .to_string_lossy()
            .into_owned();

        let children = scan_one_level(root);
        let root_node = FileNode::dir(root_name, root.to_path_buf(), children);

        Self { root: root_node }
    }

    /// Expand a directory at `path` — replaces its children with a fresh one-level scan.
    pub fn expand(&mut self, path: &Path) {
        if let Some(node) = self.find_mut(path) {
            if node.is_dir {
                let children = scan_one_level(path);
                node.children = Some(children);
            }
        }
    }

    /// Rescan a subtree rooted at `path`.
    pub fn refresh(&mut self, path: &Path) {
        self.expand(path);
    }

    /// Find a node by its path (immutable).
    pub fn find(&self, path: &Path) -> Option<&FileNode> {
        find_in_node(&self.root, path)
    }

    /// Find a node by its path (mutable).
    fn find_mut(&mut self, path: &Path) -> Option<&mut FileNode> {
        find_in_node_mut(&mut self.root, path)
    }
}

/// Scan one level of a directory, respecting `.gitignore`.
fn scan_one_level(dir: &Path) -> Vec<FileNode> {
    let mut entries = Vec::new();

    for result in WalkBuilder::new(dir)
        .max_depth(Some(1))
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build()
    {
        let Ok(entry) = result else { continue };

        // Skip the root directory itself.
        if entry.path() == dir {
            continue;
        }

        let name = entry
            .file_name()
            .to_string_lossy()
            .into_owned();
        let path = entry.path().to_path_buf();

        let node = if entry.file_type().is_some_and(|ft| ft.is_dir()) {
            FileNode::dir_lazy(name, path)
        } else {
            FileNode::file(name, path)
        };
        entries.push(node);
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
    });

    entries
}

fn find_in_node<'a>(node: &'a FileNode, target: &Path) -> Option<&'a FileNode> {
    if node.path == target {
        return Some(node);
    }
    if let Some(children) = &node.children {
        for child in children {
            if let Some(found) = find_in_node(child, target) {
                return Some(found);
            }
        }
    }
    None
}

fn find_in_node_mut<'a>(node: &'a mut FileNode, target: &Path) -> Option<&'a mut FileNode> {
    if node.path == target {
        return Some(node);
    }
    if let Some(children) = &mut node.children {
        for child in children {
            if let Some(found) = find_in_node_mut(child, target) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_tree() -> TempDir {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(tmp.path().join("README.md"), "# hi").unwrap();
        tmp
    }

    #[test]
    fn scan_lists_entries() {
        let tmp = make_tree();
        let tree = FileTree::scan(tmp.path());
        assert!(tree.root.is_dir);
        let children = tree.root.children.as_ref().unwrap();
        assert!(children.iter().any(|n| n.name == "src" && n.is_dir));
        assert!(children.iter().any(|n| n.name == "README.md" && !n.is_dir));
    }

    #[test]
    fn find_returns_node() {
        let tmp = make_tree();
        let tree = FileTree::scan(tmp.path());
        let node = tree.find(&tmp.path().join("src"));
        assert!(node.is_some());
        assert!(node.unwrap().is_dir);
    }

    #[test]
    fn expand_populates_children() {
        let tmp = make_tree();
        let mut tree = FileTree::scan(tmp.path());

        let src = tree.find(&tmp.path().join("src")).unwrap();
        assert!(src.children.is_none(), "lazy: children not loaded yet");

        tree.expand(&tmp.path().join("src"));

        let src = tree.find(&tmp.path().join("src")).unwrap();
        let children = src.children.as_ref().unwrap();
        assert!(children.iter().any(|n| n.name == "main.rs"));
    }
}
