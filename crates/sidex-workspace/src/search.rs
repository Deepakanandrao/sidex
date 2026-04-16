//! Parallel text search across workspace files.
//!
//! Uses `rayon` for parallelism, `memchr` / `regex` for matching, and the
//! `ignore` crate to respect `.gitignore` rules. Binary files are detected
//! (null-byte heuristic) and skipped automatically.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use ignore::WalkBuilder;
use memchr::memmem;
use rayon::prelude::*;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};

use crate::error::WorkspaceResult;

const BINARY_CHECK_BYTES: usize = 8192;
const DEFAULT_MAX_RESULTS: usize = 500;
const DEFAULT_MAX_FILE_SIZE: u64 = 5 * 1024 * 1024;

/// Parameters for a search query.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchQuery {
    pub pattern: String,
    #[serde(default)]
    pub is_regex: bool,
    #[serde(default = "default_true")]
    pub case_sensitive: bool,
    #[serde(default)]
    pub whole_word: bool,
    #[serde(default)]
    pub max_results: Option<usize>,
}

fn default_true() -> bool {
    true
}

/// A single match result.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub path: PathBuf,
    pub line_number: usize,
    pub line_text: String,
    pub match_start: usize,
    pub match_end: usize,
}

/// A previewed replacement edit.
#[derive(Debug, Clone, Serialize)]
pub struct FileEdit {
    pub path: PathBuf,
    pub line_number: usize,
    pub original: String,
    pub replaced: String,
}

/// Parallel text search engine.
pub struct SearchEngine;

impl SearchEngine {
    /// Search for `query` across all files under `root`, respecting `.gitignore`.
    pub fn search(root: &Path, query: &SearchQuery) -> WorkspaceResult<Vec<SearchResult>> {
        let max_results = query.max_results.unwrap_or(DEFAULT_MAX_RESULTS);
        let files = collect_files(root);

        let use_literal = !query.is_regex && query.case_sensitive && !query.whole_word;

        let literal_finder = if use_literal {
            Some(Arc::new(memmem::Finder::new(query.pattern.as_bytes())))
        } else {
            None
        };

        let re = if use_literal {
            None
        } else {
            let mut pat = if query.is_regex {
                query.pattern.clone()
            } else {
                regex::escape(&query.pattern)
            };
            if query.whole_word {
                pat = format!(r"\b{pat}\b");
            }
            Some(
                RegexBuilder::new(&pat)
                    .case_insensitive(!query.case_sensitive)
                    .build()?,
            )
        };

        let hit_count = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(AtomicBool::new(false));

        let batches: Vec<Vec<SearchResult>> = files
            .par_iter()
            .filter_map(|path| {
                if done.load(Ordering::Relaxed) {
                    return None;
                }

                let content = fs::read_to_string(path).ok()?;
                let path_buf = path.clone();
                let mut local = Vec::new();

                if let Some(ref finder) = literal_finder {
                    for (line_idx, line) in content.lines().enumerate() {
                        let bytes = line.as_bytes();
                        let mut start = 0;
                        while let Some(pos) = finder.find(&bytes[start..]) {
                            local.push(SearchResult {
                                path: path_buf.clone(),
                                line_number: line_idx + 1,
                                line_text: line.to_string(),
                                match_start: start + pos,
                                match_end: start + pos + query.pattern.len(),
                            });
                            start += pos + 1;
                            if hit_count.load(Ordering::Relaxed) + local.len() >= max_results {
                                break;
                            }
                        }
                        if hit_count.load(Ordering::Relaxed) + local.len() >= max_results {
                            break;
                        }
                    }
                } else if let Some(ref re) = re {
                    for (line_idx, line) in content.lines().enumerate() {
                        for m in re.find_iter(line) {
                            local.push(SearchResult {
                                path: path_buf.clone(),
                                line_number: line_idx + 1,
                                line_text: line.to_string(),
                                match_start: m.start(),
                                match_end: m.end(),
                            });
                            if hit_count.load(Ordering::Relaxed) + local.len() >= max_results {
                                break;
                            }
                        }
                        if hit_count.load(Ordering::Relaxed) + local.len() >= max_results {
                            break;
                        }
                    }
                }

                if local.is_empty() {
                    None
                } else {
                    let prev = hit_count.fetch_add(local.len(), Ordering::Relaxed);
                    if prev + local.len() >= max_results {
                        done.store(true, Ordering::Relaxed);
                        local.truncate(max_results.saturating_sub(prev));
                    }
                    Some(local)
                }
            })
            .collect();

        let mut results = Vec::with_capacity(max_results);
        for batch in batches {
            let remaining = max_results.saturating_sub(results.len());
            if remaining == 0 {
                break;
            }
            results.extend(batch.into_iter().take(remaining));
        }

        Ok(results)
    }

    /// Preview replacements without writing to disk.
    pub fn search_replace(
        root: &Path,
        query: &SearchQuery,
        replacement: &str,
    ) -> WorkspaceResult<Vec<FileEdit>> {
        let hits = Self::search(root, query)?;
        let mut edits = Vec::with_capacity(hits.len());

        let re = if query.is_regex {
            Some(
                RegexBuilder::new(&query.pattern)
                    .case_insensitive(!query.case_sensitive)
                    .build()?,
            )
        } else {
            None
        };

        for hit in hits {
            let replaced = if let Some(ref re) = re {
                re.replace_all(&hit.line_text, replacement).into_owned()
            } else if query.case_sensitive {
                hit.line_text.replace(&query.pattern, replacement)
            } else {
                case_insensitive_replace(&hit.line_text, &query.pattern, replacement)
            };

            edits.push(FileEdit {
                path: hit.path,
                line_number: hit.line_number,
                original: hit.line_text,
                replaced,
            });
        }

        Ok(edits)
    }
}

fn case_insensitive_replace(text: &str, pattern: &str, replacement: &str) -> String {
    let lower_text = text.to_lowercase();
    let lower_pat = pattern.to_lowercase();
    let mut result = String::with_capacity(text.len());
    let mut last = 0;

    for (idx, _) in lower_text.match_indices(&lower_pat) {
        result.push_str(&text[last..idx]);
        result.push_str(replacement);
        last = idx + pattern.len();
    }
    result.push_str(&text[last..]);
    result
}

/// Collect all searchable (non-binary, non-huge) files under `root`.
fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for result in WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build()
    {
        let Ok(entry) = result else { continue };

        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let path = entry.path();
        if let Ok(meta) = entry.metadata() {
            if meta.len() > DEFAULT_MAX_FILE_SIZE {
                continue;
            }
        }

        if is_binary(path) {
            continue;
        }

        files.push(path.to_path_buf());
    }

    files
}

fn is_binary(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; BINARY_CHECK_BYTES];
    let Ok(n) = file.read(&mut buf) else {
        return false;
    };
    buf[..n].contains(&0)
}

// ---------------------------------------------------------------------------
// Fuzzy file search (ported from src-tauri/src/commands/search.rs)
// ---------------------------------------------------------------------------

static ALWAYS_SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "dist",
    "build",
    "out",
    "__pycache__",
    ".next",
    ".cache",
];

/// A file matched by fuzzy filename search.
#[derive(Debug, Clone, Serialize)]
pub struct FileMatch {
    pub path: String,
    pub name: String,
    pub score: i64,
}

/// Options for fuzzy filename search.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FileSearchOptions {
    pub max_results: Option<usize>,
    pub include_hidden: Option<bool>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

fn should_skip_entry(entry: &walkdir::DirEntry, include_hidden: bool) -> bool {
    let name = entry.file_name().to_string_lossy();
    if !include_hidden && name.starts_with('.') {
        return true;
    }
    if entry.file_type().is_dir() && ALWAYS_SKIP_DIRS.contains(&name.as_ref()) {
        return true;
    }
    false
}

fn build_globset(patterns: &[String]) -> Option<globset::GlobSet> {
    if patterns.is_empty() {
        return None;
    }
    let mut builder = globset::GlobSetBuilder::new();
    for p in patterns {
        if let Ok(g) = globset::Glob::new(p) {
            builder.add(g);
        }
    }
    builder.build().ok()
}

#[allow(clippy::cast_possible_wrap)]
fn fuzzy_score(pattern: &[u8], target: &str) -> Option<i64> {
    if pattern.is_empty() {
        return Some(0);
    }
    let target_bytes = target.as_bytes();
    let mut pi = 0;
    let mut score: i64 = 0;
    let mut consecutive = 0i64;
    let mut prev_match = false;

    for (ti, &tc) in target_bytes.iter().enumerate() {
        if pi < pattern.len() && tc.eq_ignore_ascii_case(&pattern[pi]) {
            score += 1;
            if ti == 0 || !target_bytes[ti - 1].is_ascii_alphanumeric() {
                score += 5;
            }
            if tc == pattern[pi] {
                score += 1;
            }
            if prev_match {
                consecutive += 1;
                score += consecutive * 2;
            } else {
                consecutive = 0;
            }
            prev_match = true;
            pi += 1;
        } else {
            prev_match = false;
            consecutive = 0;
        }
    }

    if pi == pattern.len() {
        let len_penalty = (target_bytes.len() as i64 - pattern.len() as i64).min(20);
        Some(score * 100 - len_penalty)
    } else {
        None
    }
}

/// Fuzzy-search for files by name under `root`.
pub fn search_files(
    root: &Path,
    pattern: &str,
    options: Option<&FileSearchOptions>,
) -> Vec<FileMatch> {
    let max_results = options
        .and_then(|o| o.max_results)
        .unwrap_or(DEFAULT_MAX_RESULTS);
    let include_hidden = options.and_then(|o| o.include_hidden).unwrap_or(false);
    let include_set = options
        .and_then(|o| o.include.as_deref())
        .and_then(|v| if v.is_empty() { None } else { build_globset(v) });
    let exclude_set = options
        .and_then(|o| o.exclude.as_deref())
        .and_then(|v| if v.is_empty() { None } else { build_globset(v) });

    let pattern_bytes = pattern.as_bytes().to_vec();
    let mut scored: Vec<FileMatch> = Vec::with_capacity(max_results * 2);

    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .max_depth(20)
        .into_iter()
        .filter_entry(|e| !should_skip_entry(e, include_hidden))
        .filter_map(std::result::Result::ok)
    {
        if entry.file_type().is_dir() {
            continue;
        }

        let path = entry.path();

        if let Some(ref inc) = include_set {
            if !inc.is_match(path) {
                continue;
            }
        }
        if let Some(ref exc) = exclude_set {
            if exc.is_match(path) {
                continue;
            }
        }

        let name = entry.file_name().to_string_lossy();
        let Some(score) = fuzzy_score(&pattern_bytes, &name) else {
            continue;
        };

        scored.push(FileMatch {
            path: path.to_string_lossy().into_owned(),
            name: name.into_owned(),
            score,
        });
    }

    scored.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    scored.truncate(max_results);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("hello.rs"), "fn main() {\n    println!(\"Hello\");\n}\n").unwrap();
        fs::write(tmp.path().join("notes.txt"), "Hello world\nhello again\n").unwrap();
        tmp
    }

    #[test]
    fn case_sensitive_search() {
        let tmp = setup();
        let query = SearchQuery {
            pattern: "Hello".to_string(),
            is_regex: false,
            case_sensitive: true,
            whole_word: false,
            max_results: None,
        };
        let results = SearchEngine::search(tmp.path(), &query).unwrap();
        assert!(results.len() >= 2, "should match both files");
        assert!(results.iter().all(|r| r.line_text.contains("Hello")));
    }

    #[test]
    fn case_insensitive_search() {
        let tmp = setup();
        let query = SearchQuery {
            pattern: "hello".to_string(),
            is_regex: false,
            case_sensitive: false,
            whole_word: false,
            max_results: None,
        };
        let results = SearchEngine::search(tmp.path(), &query).unwrap();
        assert!(results.len() >= 3, "should find both casings");
    }

    #[test]
    fn regex_search() {
        let tmp = setup();
        let query = SearchQuery {
            pattern: r"fn\s+\w+".to_string(),
            is_regex: true,
            case_sensitive: true,
            whole_word: false,
            max_results: None,
        };
        let results = SearchEngine::search(tmp.path(), &query).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn search_replace_preview() {
        let tmp = setup();
        let query = SearchQuery {
            pattern: "Hello".to_string(),
            is_regex: false,
            case_sensitive: true,
            whole_word: false,
            max_results: None,
        };
        let edits = SearchEngine::search_replace(tmp.path(), &query, "Goodbye").unwrap();
        assert!(!edits.is_empty());
        for edit in &edits {
            assert!(edit.replaced.contains("Goodbye"));
            assert!(!edit.replaced.contains("Hello"));
        }
    }

    #[test]
    fn binary_files_are_skipped() {
        let tmp = TempDir::new().unwrap();
        let mut data = vec![0u8; 128];
        data[0] = 0; // null byte → binary
        fs::write(tmp.path().join("binary.bin"), &data).unwrap();
        fs::write(tmp.path().join("text.txt"), "needle").unwrap();

        let query = SearchQuery {
            pattern: "needle".to_string(),
            is_regex: false,
            case_sensitive: true,
            whole_word: false,
            max_results: None,
        };
        let results = SearchEngine::search(tmp.path(), &query).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].path.ends_with("text.txt"));
    }

    #[test]
    fn fuzzy_file_search() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("main.rs"), "").unwrap();
        fs::write(tmp.path().join("utils.rs"), "").unwrap();
        fs::write(tmp.path().join("readme.md"), "").unwrap();

        let opts = FileSearchOptions {
            include_hidden: Some(true),
            ..Default::default()
        };
        let results = search_files(tmp.path(), "main", Some(&opts));
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "main.rs");
    }

    #[test]
    fn fuzzy_score_exact_match() {
        let score = fuzzy_score(b"main", "main.rs");
        assert!(score.is_some());
        assert!(score.unwrap() > 0);
    }

    #[test]
    fn fuzzy_score_no_match() {
        let score = fuzzy_score(b"xyz", "main.rs");
        assert!(score.is_none());
    }
}
