//! Git log — commit history.

use std::path::Path;

use serde::Serialize;

use crate::cmd::run_git;
use crate::error::GitResult;

/// A single commit entry.
#[derive(Debug, Clone, Serialize)]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

/// A commit entry with optional graph / stat info (parent hashes, email, stats).
#[derive(Debug, Clone, Serialize)]
pub struct GraphCommit {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_hashes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_changed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insertions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletions: Option<u32>,
}

/// Fetch the last `count` commits from the repository.
pub fn get_log(repo_root: &Path, count: usize) -> GitResult<Vec<Commit>> {
    let limit = format!("-{count}");
    let output = run_git(
        repo_root,
        &["log", "--format=%H%n%h%n%an%n%aI%n%s", &limit],
    )?;
    Ok(parse_log_output(&output))
}

/// Fetch the last `count` commits that touched a specific file.
pub fn get_file_log(repo_root: &Path, path: &Path, count: usize) -> GitResult<Vec<Commit>> {
    let limit = format!("-{count}");
    let path_str = path.to_string_lossy();
    let output = run_git(
        repo_root,
        &["log", "--format=%H%n%h%n%an%n%aI%n%s", &limit, "--", &path_str],
    )?;
    Ok(parse_log_output(&output))
}

/// Fetch a detailed log with parent hashes, emails, and shortstat info.
pub fn get_log_graph(repo_root: &Path, count: usize) -> GitResult<Vec<GraphCommit>> {
    let limit = format!("-{count}");
    let output = run_git(
        repo_root,
        &[
            "log",
            "--format=%H%n%P%n%s%n%an%n%ae%n%aI",
            "--shortstat",
            &limit,
        ],
    )?;

    let mut entries = Vec::new();
    let mut lines = output.lines().peekable();

    while lines.peek().is_some() {
        let hash = match lines.next() {
            Some(h) if !h.is_empty() => h.to_string(),
            _ => break,
        };
        let parents_line = lines.next().unwrap_or("");
        let subject = lines.next().unwrap_or("").to_string();
        let author = lines.next().unwrap_or("").to_string();
        let email = lines.next().unwrap_or("").to_string();
        let date = lines.next().unwrap_or("").to_string();

        let mut files_changed: Option<u32> = None;
        let mut insertions: Option<u32> = None;
        let mut deletions: Option<u32> = None;

        while let Some(&next) = lines.peek() {
            if next.is_empty() {
                lines.next();
                continue;
            }
            if next.contains("file") && next.contains("changed") {
                let stat_line = lines.next().unwrap_or("");
                for part in stat_line.split(',') {
                    let part = part.trim();
                    if part.contains("file") {
                        files_changed = part.split_whitespace().next().and_then(|n| n.parse().ok());
                    } else if part.contains("insertion") {
                        insertions = part.split_whitespace().next().and_then(|n| n.parse().ok());
                    } else if part.contains("deletion") {
                        deletions = part.split_whitespace().next().and_then(|n| n.parse().ok());
                    }
                }
                break;
            }
            break;
        }

        let parent_hashes: Vec<String> = parents_line
            .split_whitespace()
            .map(std::string::ToString::to_string)
            .collect();

        entries.push(GraphCommit {
            hash,
            message: subject,
            author,
            date,
            parent_hashes: Some(parent_hashes),
            email: if email.is_empty() { None } else { Some(email) },
            files_changed,
            insertions,
            deletions,
        });
    }

    Ok(entries)
}

fn parse_log_output(output: &str) -> Vec<Commit> {
    let lines: Vec<&str> = output.lines().collect();
    lines
        .chunks(5)
        .filter(|chunk| chunk.len() == 5)
        .map(|chunk| Commit {
            hash: chunk[0].to_string(),
            short_hash: chunk[1].to_string(),
            author: chunk[2].to_string(),
            date: chunk[3].to_string(),
            message: chunk[4].to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_log_output_works() {
        let output = "abc123def456\nabc123d\nAlice\n2025-01-01T00:00:00+00:00\ninitial commit\n\
                       def456abc789\ndef456a\nBob\n2025-01-02T00:00:00+00:00\nfix bug\n";
        let commits = parse_log_output(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].author, "Alice");
        assert_eq!(commits[1].message, "fix bug");
        assert_eq!(commits[0].short_hash, "abc123d");
    }

    #[test]
    fn parse_log_handles_partial_chunk() {
        let output = "hash\nshort\nauthor\n";
        let commits = parse_log_output(output);
        assert!(commits.is_empty(), "incomplete chunk should be skipped");
    }
}
