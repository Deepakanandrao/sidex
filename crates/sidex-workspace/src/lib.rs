//! Workspace management — file tree, watcher, search, indexing, path utilities for `SideX`.

pub mod error;
pub mod file_ops;
pub mod file_tree;
pub mod index;
pub mod path_util;
pub mod search;
pub mod watcher;
pub mod workspace;

pub use error::{WorkspaceError, WorkspaceResult};
pub use file_ops::{DirEntry, FileStat};
pub use file_tree::{FileNode, FileTree};
pub use index::{InvertedIndex, IndexOptions, IndexSearchOptions, IndexSearchResult, IndexStats};
pub use path_util::PathInfo;
pub use search::{FileEdit, FileMatch, FileSearchOptions, SearchEngine, SearchQuery, SearchResult};
pub use watcher::{FileEvent, FileEventKind, FileWatcher};
pub use workspace::Workspace;
