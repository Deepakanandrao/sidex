//! Git diff — file diffs, staged diffs, and line-level diff info.

use std::path::Path;

use serde::Serialize;

use crate::cmd::run_git;
use crate::error::GitResult;

/// The kind of change on a single line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LineDiffKind {
    Added,
    Removed,
    Modified,
}

/// A single line-level diff entry (for gutter decorations).
#[derive(Debug, Clone, Serialize)]
pub struct LineDiff {
    pub line_number: usize,
    pub kind: LineDiffKind,
}

/// Get the full diff for a specific file (unstaged changes).
pub fn get_diff(repo_root: &Path, path: &Path) -> GitResult<String> {
    let path_str = path.to_string_lossy();
    let output = run_git(repo_root, &["diff", "--", &path_str])?;
    Ok(output)
}

/// Get the staged diff for a specific file.
pub fn get_diff_staged(repo_root: &Path, path: &Path) -> GitResult<String> {
    let path_str = path.to_string_lossy();
    let output = run_git(repo_root, &["diff", "--staged", "--", &path_str])?;
    Ok(output)
}

/// Parse `git diff` output into line-level diffs for gutter decorations.
pub fn get_line_diffs(repo_root: &Path, path: &Path) -> GitResult<Vec<LineDiff>> {
    let path_str = path.to_string_lossy();
    let output = run_git(
        repo_root,
        &["diff", "--unified=0", "--no-color", "--", &path_str],
    )?;
    Ok(parse_unified_diff(&output))
}

/// Parse a unified diff (with `--unified=0`) into `LineDiff` entries.
fn parse_unified_diff(diff: &str) -> Vec<LineDiff> {
    let mut diffs = Vec::new();

    for line in diff.lines() {
        // Hunk headers: @@ -old_start[,old_count] +new_start[,new_count] @@
        if let Some(hunk) = line.strip_prefix("@@ ") {
            if let Some((removed, added)) = parse_hunk_header(hunk) {
                if removed.count > 0 && added.count == 0 {
                    // Lines were deleted before `added.start`
                    diffs.push(LineDiff {
                        line_number: added.start.max(1),
                        kind: LineDiffKind::Removed,
                    });
                } else if removed.count == 0 && added.count > 0 {
                    for i in 0..added.count {
                        diffs.push(LineDiff {
                            line_number: added.start + i,
                            kind: LineDiffKind::Added,
                        });
                    }
                } else {
                    for i in 0..added.count {
                        diffs.push(LineDiff {
                            line_number: added.start + i,
                            kind: LineDiffKind::Modified,
                        });
                    }
                }
            }
        }
    }

    diffs
}

struct HunkRange {
    start: usize,
    count: usize,
}

fn parse_hunk_header(header: &str) -> Option<(HunkRange, HunkRange)> {
    // Format: "-old_start[,old_count] +new_start[,new_count] @@..."
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let removed = parse_range(parts[0].strip_prefix('-')?)?;
    let added = parse_range(parts[1].strip_prefix('+')?)?;

    Some((removed, added))
}

fn parse_range(s: &str) -> Option<HunkRange> {
    if let Some((start, count)) = s.split_once(',') {
        Some(HunkRange {
            start: start.parse().ok()?,
            count: count.parse().ok()?,
        })
    } else {
        Some(HunkRange {
            start: s.parse().ok()?,
            count: 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_added_lines() {
        let diff = "@@ -0,0 +1,3 @@\n+a\n+b\n+c\n";
        let diffs = parse_unified_diff(diff);
        assert_eq!(diffs.len(), 3);
        assert!(diffs.iter().all(|d| d.kind == LineDiffKind::Added));
        assert_eq!(diffs[0].line_number, 1);
        assert_eq!(diffs[2].line_number, 3);
    }

    #[test]
    fn parse_removed_lines() {
        let diff = "@@ -5,2 +5,0 @@\n-old1\n-old2\n";
        let diffs = parse_unified_diff(diff);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].kind, LineDiffKind::Removed);
        assert_eq!(diffs[0].line_number, 5);
    }

    #[test]
    fn parse_modified_lines() {
        let diff = "@@ -10,2 +10,2 @@\n-old\n-old2\n+new\n+new2\n";
        let diffs = parse_unified_diff(diff);
        assert_eq!(diffs.len(), 2);
        assert!(diffs.iter().all(|d| d.kind == LineDiffKind::Modified));
    }
}
