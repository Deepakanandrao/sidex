//! Git integration for `SideX`.
//!
//! All operations shell out to the `git` CLI via `std::process::Command`.

pub mod blame;
mod cmd;
pub mod diff;
pub mod error;
pub mod log;
pub mod operations;
pub mod repo;
pub mod status;

pub use blame::BlameLine;
pub use diff::{LineDiff, LineDiffKind};
pub use error::{GitError, GitResult};
pub use log::{Commit, GraphCommit};
pub use operations::{
    branches, checkout, clone, commit, create_branch, delete_branch, fetch, init, pull, push,
    remote_list, run, show_file, stage, stash, stash_action, stash_drop, stash_list, stash_pop,
    unstage, GitBranch, GitRemote, StashAction,
};
pub use repo::{current_branch, find_repo_root, is_git_repo, remotes};
pub use status::{FileStatus, StatusEntry};
