//! Git operations — stage, unstage, commit, push, pull, checkout, branch, stash, fetch, clone, show, run.

use std::path::Path;

use serde::Serialize;

use crate::cmd::{git_command, run_git};
use crate::error::{GitError, GitResult};

/// A branch entry from `git branch -a`.
#[derive(Debug, Clone, Serialize)]
pub struct GitBranch {
    pub name: String,
    pub current: bool,
    pub remote: bool,
}

/// A remote entry from `git remote -v`.
#[derive(Debug, Clone, Serialize)]
pub struct GitRemote {
    pub name: String,
    pub url: String,
    pub remote_type: String,
}

/// Stash sub-command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StashAction {
    Push,
    Pop,
    List,
    Drop,
}

impl StashAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::Pop => "pop",
            Self::List => "list",
            Self::Drop => "drop",
        }
    }
}

/// Stage files.
pub fn stage(repo_root: &Path, paths: &[&Path]) -> GitResult<()> {
    let mut args: Vec<&str> = vec!["add", "--"];
    let strs: Vec<String> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    let refs: Vec<&str> = strs.iter().map(String::as_str).collect();
    args.extend(refs);
    run_git(repo_root, &args)?;
    Ok(())
}

/// Unstage files (reset HEAD).
pub fn unstage(repo_root: &Path, paths: &[&Path]) -> GitResult<()> {
    let mut args: Vec<&str> = vec!["reset", "HEAD", "--"];
    let strs: Vec<String> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    let refs: Vec<&str> = strs.iter().map(String::as_str).collect();
    args.extend(refs);
    run_git(repo_root, &args)?;
    Ok(())
}

/// Commit staged changes, returning the new commit hash.
pub fn commit(repo_root: &Path, message: &str) -> GitResult<String> {
    run_git(repo_root, &["commit", "-m", message])?;
    let hash = run_git(repo_root, &["rev-parse", "HEAD"])?;
    Ok(hash.trim().to_string())
}

/// Push to a remote. Pass `None` to use defaults.
pub fn push(repo_root: &Path, remote: Option<&str>, branch: Option<&str>) -> GitResult<String> {
    let mut args = vec!["push"];
    if let Some(r) = remote {
        args.push(r);
    }
    if let Some(b) = branch {
        args.push(b);
    }
    run_git(repo_root, &args)
}

/// Pull from a remote. Pass `None` to use defaults.
pub fn pull(repo_root: &Path, remote: Option<&str>, branch: Option<&str>) -> GitResult<String> {
    let mut args = vec!["pull"];
    if let Some(r) = remote {
        args.push(r);
    }
    if let Some(b) = branch {
        args.push(b);
    }
    run_git(repo_root, &args)
}

/// Fetch from a remote. Pass `None` to fetch from the default remote.
pub fn fetch(repo_root: &Path, remote: Option<&str>) -> GitResult<String> {
    let mut args = vec!["fetch"];
    if let Some(r) = remote {
        args.push(r);
    }
    run_git(repo_root, &args)
}

/// Checkout a branch.
pub fn checkout(repo_root: &Path, branch: &str) -> GitResult<()> {
    run_git(repo_root, &["checkout", branch])?;
    Ok(())
}

/// Create a new branch and switch to it, optionally from a start point.
pub fn create_branch(repo_root: &Path, name: &str, start_point: Option<&str>) -> GitResult<()> {
    let mut args = vec!["checkout", "-b", name];
    if let Some(sp) = start_point {
        args.push(sp);
    }
    run_git(repo_root, &args)?;
    Ok(())
}

/// Delete a local branch.
pub fn delete_branch(repo_root: &Path, name: &str) -> GitResult<()> {
    run_git(repo_root, &["branch", "-d", name])?;
    Ok(())
}

/// List all branches (local and remote).
pub fn branches(repo_root: &Path) -> GitResult<Vec<GitBranch>> {
    let output = run_git(repo_root, &["branch", "-a"])?;
    let result = output
        .lines()
        .filter(|l| !l.is_empty())
        .filter(|l| !l.contains("->"))
        .map(|line| {
            let current = line.starts_with('*');
            let name = line.trim_start_matches('*').trim().to_string();
            let remote = name.starts_with("remotes/");
            let name = name.trim_start_matches("remotes/").to_string();
            GitBranch {
                name,
                current,
                remote,
            }
        })
        .collect();
    Ok(result)
}

/// Stash uncommitted changes with an optional message.
pub fn stash(repo_root: &Path, message: Option<&str>) -> GitResult<String> {
    let mut args = vec!["stash", "push"];
    if let Some(m) = message {
        args.push("-m");
        args.push(m);
    }
    run_git(repo_root, &args)
}

/// Pop the most recent stash.
pub fn stash_pop(repo_root: &Path) -> GitResult<String> {
    run_git(repo_root, &["stash", "pop"])
}

/// List stash entries.
pub fn stash_list(repo_root: &Path) -> GitResult<String> {
    run_git(repo_root, &["stash", "list"])
}

/// Drop the most recent stash entry.
pub fn stash_drop(repo_root: &Path) -> GitResult<String> {
    run_git(repo_root, &["stash", "drop"])
}

/// Run a stash sub-command with an optional message (for push).
pub fn stash_action(repo_root: &Path, action: StashAction, message: Option<&str>) -> GitResult<String> {
    let mut args = vec!["stash", action.as_str()];
    if action == StashAction::Push {
        if let Some(m) = message {
            args.push("-m");
            args.push(m);
        }
    }
    run_git(repo_root, &args)
}

/// Initialize a new git repository.
pub fn init(repo_root: &Path) -> GitResult<()> {
    run_git(repo_root, &["init"])?;
    Ok(())
}

/// List remotes with their URLs.
pub fn remote_list(repo_root: &Path) -> GitResult<Vec<GitRemote>> {
    let output = run_git(repo_root, &["remote", "-v"])?;
    let mut remotes = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let remote_type = parts[2].trim_matches(|c| c == '(' || c == ')').to_string();
            remotes.push(GitRemote {
                name: parts[0].to_string(),
                url: parts[1].to_string(),
                remote_type,
            });
        }
    }
    Ok(remotes)
}

/// Show the HEAD version of a file as raw bytes.
pub fn show_file(repo_root: &Path, file: &str) -> GitResult<Vec<u8>> {
    let rev_file = format!("HEAD:{file}");
    let output = git_command()
        .current_dir(repo_root)
        .args(["show", &rev_file])
        .output()
        .map_err(GitError::Exec)?;

    if output.status.success() {
        Ok(output.stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(GitError::Command {
            message: format!("git show error: {}", stderr.trim()),
        })
    }
}

/// Clone a repository. Performs a `--no-checkout` clone followed by a hooks-disabled checkout.
pub fn clone(url: &str, dest: &Path) -> GitResult<()> {
    let dest_str = dest.to_string_lossy();

    if dest
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(GitError::Command {
            message: "clone destination must not contain '..'".to_string(),
        });
    }

    let output = git_command()
        .args(["clone", "--no-checkout", url, &dest_str])
        .output()
        .map_err(GitError::Exec)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::Command {
            message: format!("git clone error: {}", stderr.trim()),
        });
    }

    let checkout_out = git_command()
        .current_dir(dest)
        .args(["-c", "core.hooksPath=/dev/null", "checkout"])
        .output()
        .map_err(GitError::Exec)?;

    if checkout_out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&checkout_out.stderr);
        Err(GitError::Command {
            message: format!("git checkout error: {}", stderr.trim()),
        })
    }
}

const BLOCKED_GIT_FLAGS: &[&str] = &[
    "-c",
    "--exec",
    "--upload-pack",
    "--receive-pack",
    "--config",
    "--exec-path",
];

const ALLOWED_GIT_SUBCOMMANDS: &[&str] = &[
    "add", "am", "apply", "archive", "bisect", "blame", "branch", "cat-file",
    "cherry-pick", "checkout", "clean", "clone", "commit", "describe", "diff",
    "diff-tree", "fetch", "for-each-ref", "format-patch", "gc", "grep",
    "hash-object", "init", "log", "ls-files", "ls-remote", "ls-tree", "merge",
    "pack-refs", "prune", "pull", "push", "rebase", "reflog", "remote", "reset",
    "revert", "rev-parse", "shortlog", "show", "stash", "status", "submodule",
    "tag", "worktree",
];

fn validate_git_args(args: &[&str]) -> GitResult<()> {
    let subcommand = args.first().copied().unwrap_or("");
    if !ALLOWED_GIT_SUBCOMMANDS.contains(&subcommand) {
        return Err(GitError::Command {
            message: format!("git subcommand '{subcommand}' is not allowed"),
        });
    }
    for arg in args.iter().skip(1) {
        let lower = arg.to_lowercase();
        for blocked in BLOCKED_GIT_FLAGS {
            if lower == *blocked || lower.starts_with(&format!("{blocked}=")) {
                return Err(GitError::Command {
                    message: format!("git flag '{arg}' is not allowed"),
                });
            }
        }
    }
    Ok(())
}

/// Run an arbitrary (validated) git subcommand. Only allowed subcommands are
/// accepted, and dangerous flags are blocked.
pub fn run(repo_root: &Path, args: &[&str]) -> GitResult<String> {
    validate_git_args(args)?;
    run_git(repo_root, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn init_repo_with_commit() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path();
        std::process::Command::new("git").current_dir(p).args(["init", "-b", "main"]).output().unwrap();
        std::process::Command::new("git").current_dir(p).args(["config", "user.email", "t@t.com"]).output().unwrap();
        std::process::Command::new("git").current_dir(p).args(["config", "user.name", "T"]).output().unwrap();
        fs::write(p.join("init.txt"), "init").unwrap();
        std::process::Command::new("git").current_dir(p).args(["add", "."]).output().unwrap();
        std::process::Command::new("git").current_dir(p).args(["commit", "-m", "init"]).output().unwrap();
        tmp
    }

    #[test]
    fn stage_and_commit() {
        let tmp = init_repo_with_commit();
        fs::write(tmp.path().join("new.txt"), "data").unwrap();

        let new_path = Path::new("new.txt");
        stage(tmp.path(), &[new_path]).unwrap();
        let hash = commit(tmp.path(), "add new file").unwrap();
        assert!(!hash.is_empty());
    }

    #[test]
    fn create_and_checkout_branch() {
        let tmp = init_repo_with_commit();
        create_branch(tmp.path(), "feature", None).unwrap();

        let branch = crate::repo::current_branch(tmp.path()).unwrap();
        assert_eq!(branch, "feature");

        checkout(tmp.path(), "main").unwrap();
        let branch = crate::repo::current_branch(tmp.path()).unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn list_branches() {
        let tmp = init_repo_with_commit();
        create_branch(tmp.path(), "dev", None).unwrap();
        checkout(tmp.path(), "main").unwrap();

        let all = branches(tmp.path()).unwrap();
        assert!(all.iter().any(|b| b.name == "main" && b.current));
        assert!(all.iter().any(|b| b.name == "dev" && !b.current));
    }

    #[test]
    fn delete_branch_works() {
        let tmp = init_repo_with_commit();
        create_branch(tmp.path(), "temp", None).unwrap();
        checkout(tmp.path(), "main").unwrap();
        delete_branch(tmp.path(), "temp").unwrap();
        let all = branches(tmp.path()).unwrap();
        assert!(!all.iter().any(|b| b.name == "temp"));
    }

    #[test]
    fn validate_git_args_blocks_dangerous_flags() {
        assert!(validate_git_args(&["rm", "-rf", "/"]).is_err());
        assert!(validate_git_args(&["status"]).is_ok());
        assert!(validate_git_args(&["log", "--config=x"]).is_err());
    }

    #[test]
    fn run_validated() {
        let tmp = init_repo_with_commit();
        let output = run(tmp.path(), &["status", "--porcelain"]).unwrap();
        assert!(output.is_empty() || output.contains(' '));
    }

    #[test]
    fn show_file_works() {
        let tmp = init_repo_with_commit();
        let content = show_file(tmp.path(), "init.txt").unwrap();
        assert_eq!(String::from_utf8(content).unwrap(), "init");
    }

    #[test]
    fn init_works() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();
        assert!(tmp.path().join(".git").exists());
    }
}
