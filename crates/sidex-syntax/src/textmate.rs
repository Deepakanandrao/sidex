//! `TextMate` grammar support for languages without tree-sitter grammars.
//!
//! Provides a regex-based tokenizer that loads `.tmLanguage.json` or `.plist`
//! grammar definitions and tokenizes source lines using `TextMate` scope rules.
//! This serves as a fallback highlighting engine for languages that lack a
//! tree-sitter parser.

use std::collections::HashMap;
use std::path::Path;

use regex::Regex;
use serde::Deserialize;

/// Information about a single token produced by the tokenizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenInfo {
    /// Start byte offset within the line.
    pub start: usize,
    /// End byte offset within the line.
    pub end: usize,
    /// Stack of `TextMate` scope names, outermost first.
    pub scopes: Vec<String>,
}

/// Persistent state carried between lines during tokenization.
#[derive(Debug, Clone)]
pub struct TokenizerState {
    /// Stack of active rule scopes. Each entry is `(scope_name, rule_index)`.
    pub rule_stack: Vec<(String, usize)>,
}

impl Default for TokenizerState {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenizerState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            rule_stack: Vec::new(),
        }
    }
}

/// A compiled `TextMate` grammar loaded from a `.tmLanguage.json` or `.plist` file.
#[derive(Debug, Clone)]
pub struct TextMateGrammar {
    /// Top-level scope name (e.g. `"source.rust"`).
    pub scope_name: String,
    /// File extensions this grammar applies to.
    pub file_types: Vec<String>,
    /// Top-level patterns.
    pub patterns: Vec<Pattern>,
    /// Named repository rules that can be referenced via `$self`, `$base`, or
    /// `#name` includes.
    pub repository: HashMap<String, RepositoryRule>,
}

/// A single pattern rule in a `TextMate` grammar.
#[derive(Debug, Clone)]
pub enum Pattern {
    /// A single-line match rule.
    Match(MatchRule),
    /// A multi-line begin/end rule.
    BeginEnd(BeginEndRule),
    /// A multi-line begin/while rule (continues as long as `while` matches).
    BeginWhile(BeginWhileRule),
    /// An include reference to a repository rule or another grammar.
    Include(IncludeRef),
}

/// A single-line match pattern.
#[derive(Debug, Clone)]
pub struct MatchRule {
    pub regex: String,
    pub scope: Option<String>,
    pub captures: HashMap<usize, String>,
}

/// A begin/end multi-line pattern.
#[derive(Debug, Clone)]
pub struct BeginEndRule {
    pub begin: String,
    pub end: String,
    pub scope: Option<String>,
    pub begin_captures: HashMap<usize, String>,
    pub end_captures: HashMap<usize, String>,
    pub patterns: Vec<Pattern>,
}

/// A begin/while multi-line pattern.
#[derive(Debug, Clone)]
pub struct BeginWhileRule {
    pub begin: String,
    pub while_pattern: String,
    pub scope: Option<String>,
    pub begin_captures: HashMap<usize, String>,
    pub while_captures: HashMap<usize, String>,
    pub patterns: Vec<Pattern>,
}

/// An include reference (either `$self`, `$base`, or `#repo-name`).
#[derive(Debug, Clone)]
pub enum IncludeRef {
    /// `$self` — include the current grammar's patterns.
    SelfRef,
    /// `$base` — include the base grammar's patterns.
    BaseRef,
    /// `#name` — include a named repository rule.
    Repository(String),
    /// `scope.name` — include patterns from another grammar.
    External(String),
}

/// A named rule in the grammar's repository.
#[derive(Debug, Clone)]
pub struct RepositoryRule {
    pub patterns: Vec<Pattern>,
}

/// Errors that can occur when loading a `TextMate` grammar.
#[derive(Debug, thiserror::Error)]
pub enum TextMateError {
    #[error("failed to read grammar file: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid JSON grammar: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid plist grammar: {0}")]
    Plist(#[from] plist::Error),
    #[error("invalid regex in grammar: {pattern}")]
    InvalidRegex {
        pattern: String,
        #[source]
        source: regex::Error,
    },
    #[error("unsupported grammar format")]
    UnsupportedFormat,
}

/// Raw JSON/plist structure matching the tmLanguage schema.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGrammar {
    scope_name: Option<String>,
    #[serde(default)]
    file_types: Vec<String>,
    #[serde(default)]
    patterns: Vec<RawPattern>,
    #[serde(default)]
    repository: HashMap<String, RawRepo>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawPattern {
    #[serde(rename = "match")]
    match_regex: Option<String>,
    begin: Option<String>,
    end: Option<String>,
    #[serde(rename = "while")]
    while_regex: Option<String>,
    name: Option<String>,
    #[serde(default, rename = "contentName")]
    _content_name: Option<String>,
    #[serde(default)]
    captures: HashMap<String, RawCaptureName>,
    #[serde(default, rename = "beginCaptures")]
    begin_captures: HashMap<String, RawCaptureName>,
    #[serde(default, rename = "endCaptures")]
    end_captures: HashMap<String, RawCaptureName>,
    #[serde(default, rename = "whileCaptures")]
    while_captures: HashMap<String, RawCaptureName>,
    #[serde(default)]
    patterns: Vec<RawPattern>,
    include: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawCaptureName {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawRepo {
    #[serde(default)]
    patterns: Vec<RawPattern>,
    #[serde(rename = "match")]
    match_regex: Option<String>,
    begin: Option<String>,
    end: Option<String>,
    name: Option<String>,
    #[serde(default)]
    captures: HashMap<String, RawCaptureName>,
    #[serde(default, rename = "beginCaptures")]
    begin_captures: HashMap<String, RawCaptureName>,
    #[serde(default, rename = "endCaptures")]
    end_captures: HashMap<String, RawCaptureName>,
    include: Option<String>,
}

impl TextMateGrammar {
    /// Loads a grammar from a `.tmLanguage.json` file.
    pub fn from_json(json: &str) -> Result<Self, TextMateError> {
        let raw: RawGrammar = serde_json::from_str(json)?;
        Ok(Self::from_raw(raw))
    }

    /// Loads a grammar from a plist (`.tmLanguage` / `.plist`) file.
    pub fn from_plist(data: &[u8]) -> Result<Self, TextMateError> {
        let raw: RawGrammar = plist::from_bytes(data)?;
        Ok(Self::from_raw(raw))
    }

    /// Loads a grammar by detecting the format from the file extension.
    pub fn from_file(path: &Path) -> Result<Self, TextMateError> {
        let data = std::fs::read(path)?;
        match path.extension().and_then(|e| e.to_str()) {
            Some("json") => Self::from_json(&String::from_utf8_lossy(&data)),
            Some("plist" | "tmLanguage") => Self::from_plist(&data),
            _ => Err(TextMateError::UnsupportedFormat),
        }
    }

    fn from_raw(raw: RawGrammar) -> Self {
        let patterns = raw.patterns.into_iter().map(convert_pattern).collect();
        let repository = raw
            .repository
            .into_iter()
            .map(|(name, repo)| (name, convert_repo(repo)))
            .collect();

        Self {
            scope_name: raw.scope_name.unwrap_or_default(),
            file_types: raw.file_types,
            patterns,
            repository,
        }
    }
}

fn convert_captures(raw: &HashMap<String, RawCaptureName>) -> HashMap<usize, String> {
    raw.iter()
        .filter_map(|(k, v)| {
            let idx = k.parse::<usize>().ok()?;
            let name = v.name.clone()?;
            Some((idx, name))
        })
        .collect()
}

fn convert_pattern(raw: RawPattern) -> Pattern {
    if let Some(include) = raw.include {
        return Pattern::Include(parse_include(&include));
    }

    if let Some(regex) = raw.match_regex {
        return Pattern::Match(MatchRule {
            regex,
            scope: raw.name,
            captures: convert_captures(&raw.captures),
        });
    }

    if let Some(begin) = raw.begin {
        if let Some(while_pat) = raw.while_regex {
            return Pattern::BeginWhile(BeginWhileRule {
                begin,
                while_pattern: while_pat,
                scope: raw.name,
                begin_captures: convert_captures(&raw.begin_captures),
                while_captures: convert_captures(&raw.while_captures),
                patterns: raw.patterns.into_iter().map(convert_pattern).collect(),
            });
        }
        if let Some(end) = raw.end {
            return Pattern::BeginEnd(BeginEndRule {
                begin,
                end,
                scope: raw.name,
                begin_captures: convert_captures(&raw.begin_captures),
                end_captures: convert_captures(&raw.end_captures),
                patterns: raw.patterns.into_iter().map(convert_pattern).collect(),
            });
        }
    }

    Pattern::Match(MatchRule {
        regex: String::new(),
        scope: raw.name,
        captures: HashMap::new(),
    })
}

fn convert_repo(raw: RawRepo) -> RepositoryRule {
    let mut patterns = Vec::new();

    if let Some(include) = raw.include {
        patterns.push(Pattern::Include(parse_include(&include)));
    } else if let Some(regex) = raw.match_regex {
        patterns.push(Pattern::Match(MatchRule {
            regex,
            scope: raw.name.clone(),
            captures: convert_captures(&raw.captures),
        }));
    } else if let Some(begin) = raw.begin {
        if let Some(end) = raw.end {
            patterns.push(Pattern::BeginEnd(BeginEndRule {
                begin,
                end,
                scope: raw.name.clone(),
                begin_captures: convert_captures(&raw.begin_captures),
                end_captures: convert_captures(&raw.end_captures),
                patterns: raw.patterns.iter().cloned().map(convert_pattern).collect(),
            }));
        }
    }

    for p in raw.patterns {
        patterns.push(convert_pattern(p));
    }

    RepositoryRule { patterns }
}

fn parse_include(s: &str) -> IncludeRef {
    match s {
        "$self" => IncludeRef::SelfRef,
        "$base" => IncludeRef::BaseRef,
        s if s.starts_with('#') => IncludeRef::Repository(s[1..].to_owned()),
        other => IncludeRef::External(other.to_owned()),
    }
}

/// Tokenizer that processes source lines using a [`TextMateGrammar`].
pub struct TextMateTokenizer<'g> {
    grammar: &'g TextMateGrammar,
}

impl<'g> TextMateTokenizer<'g> {
    /// Creates a new tokenizer backed by the given grammar.
    #[must_use]
    pub fn new(grammar: &'g TextMateGrammar) -> Self {
        Self { grammar }
    }

    /// Tokenizes a single line, mutating `state` for multi-line constructs.
    ///
    /// Returns a list of [`TokenInfo`] spans covering the line.
    pub fn tokenize_line(&self, line: &str, state: &mut TokenizerState) -> Vec<TokenInfo> {
        let mut tokens = Vec::new();
        let mut pos = 0;
        let base_scopes: Vec<String> = std::iter::once(self.grammar.scope_name.clone())
            .chain(state.rule_stack.iter().map(|(s, _)| s.clone()))
            .collect();

        if !state.rule_stack.is_empty() {
            let (scope, rule_idx) = state.rule_stack.last().unwrap().clone();
            if let Some(rule) = self.find_begin_end_rule(rule_idx) {
                if let Ok(re) = Regex::new(&rule.end) {
                    if let Some(m) = re.find(line) {
                        if pos < m.start() {
                            let mut scopes = base_scopes.clone();
                            scopes.push(scope.clone());
                            tokens.push(TokenInfo {
                                start: pos,
                                end: m.start(),
                                scopes,
                            });
                        }
                        let mut scopes = base_scopes.clone();
                        scopes.push(scope);
                        tokens.push(TokenInfo {
                            start: m.start(),
                            end: m.end(),
                            scopes,
                        });
                        pos = m.end();
                        state.rule_stack.pop();
                    } else {
                        let mut scopes = base_scopes.clone();
                        scopes.push(scope);
                        tokens.push(TokenInfo {
                            start: 0,
                            end: line.len(),
                            scopes,
                        });
                        return tokens;
                    }
                }
            }
        }

        while pos < line.len() {
            let remaining = &line[pos..];
            if let Some((info, advance)) =
                self.try_match_patterns(&self.grammar.patterns, remaining, pos, &base_scopes, 0)
            {
                tokens.extend(info);
                pos += advance;
            } else {
                let next_end = (pos + 1).min(line.len());
                tokens.push(TokenInfo {
                    start: pos,
                    end: next_end,
                    scopes: base_scopes.clone(),
                });
                pos = next_end;
            }
        }

        merge_adjacent_tokens(&mut tokens);
        tokens
    }

    fn try_match_patterns(
        &self,
        patterns: &[Pattern],
        text: &str,
        offset: usize,
        base_scopes: &[String],
        depth: usize,
    ) -> Option<(Vec<TokenInfo>, usize)> {
        if depth > 8 {
            return None;
        }

        let mut best: Option<(usize, Vec<TokenInfo>, usize)> = None;

        for pattern in patterns {
            match pattern {
                Pattern::Match(rule) => {
                    if rule.regex.is_empty() {
                        continue;
                    }
                    let Ok(re) = Regex::new(&rule.regex) else {
                        continue;
                    };
                    if let Some(m) = re.find(text) {
                        if m.start() == 0 && m.end() > 0 {
                            let start_pos = best.as_ref().map_or(usize::MAX, |b| b.0);
                            if m.start() < start_pos {
                                let mut scopes = base_scopes.to_vec();
                                if let Some(ref s) = rule.scope {
                                    scopes.push(s.clone());
                                }
                                let info = vec![TokenInfo {
                                    start: offset,
                                    end: offset + m.end(),
                                    scopes,
                                }];
                                best = Some((m.start(), info, m.end()));
                            }
                        }
                    }
                }
                Pattern::BeginEnd(rule) => {
                    let Ok(re) = Regex::new(&rule.begin) else {
                        continue;
                    };
                    if let Some(m) = re.find(text) {
                        if m.start() == 0 && m.end() > 0 {
                            let start_pos = best.as_ref().map_or(usize::MAX, |b| b.0);
                            if m.start() < start_pos {
                                let mut scopes = base_scopes.to_vec();
                                if let Some(ref s) = rule.scope {
                                    scopes.push(s.clone());
                                }
                                let info = vec![TokenInfo {
                                    start: offset,
                                    end: offset + m.end(),
                                    scopes,
                                }];
                                best = Some((m.start(), info, m.end()));
                            }
                        }
                    }
                }
                Pattern::BeginWhile(rule) => {
                    let Ok(re) = Regex::new(&rule.begin) else {
                        continue;
                    };
                    if let Some(m) = re.find(text) {
                        if m.start() == 0 && m.end() > 0 {
                            let start_pos = best.as_ref().map_or(usize::MAX, |b| b.0);
                            if m.start() < start_pos {
                                let mut scopes = base_scopes.to_vec();
                                if let Some(ref s) = rule.scope {
                                    scopes.push(s.clone());
                                }
                                let info = vec![TokenInfo {
                                    start: offset,
                                    end: offset + m.end(),
                                    scopes,
                                }];
                                best = Some((m.start(), info, m.end()));
                            }
                        }
                    }
                }
                Pattern::Include(inc) => {
                    let patterns = self.resolve_include(inc);
                    if let Some(result) =
                        self.try_match_patterns(&patterns, text, offset, base_scopes, depth + 1)
                    {
                        let start_pos = best.as_ref().map_or(usize::MAX, |b| b.0);
                        if 0 < start_pos {
                            best = Some((0, result.0, result.1));
                        }
                    }
                }
            }
        }

        best.map(|(_, info, advance)| (info, advance))
    }

    fn resolve_include(&self, inc: &IncludeRef) -> Vec<Pattern> {
        match inc {
            IncludeRef::SelfRef | IncludeRef::BaseRef => self.grammar.patterns.clone(),
            IncludeRef::Repository(name) => self
                .grammar
                .repository
                .get(name)
                .map_or_else(Vec::new, |r| r.patterns.clone()),
            IncludeRef::External(_) => Vec::new(),
        }
    }

    fn find_begin_end_rule(&self, _rule_idx: usize) -> Option<&BeginEndRule> {
        for pattern in &self.grammar.patterns {
            if let Pattern::BeginEnd(rule) = pattern {
                return Some(rule);
            }
        }
        None
    }
}

/// Merges adjacent tokens with identical scope stacks.
fn merge_adjacent_tokens(tokens: &mut Vec<TokenInfo>) {
    if tokens.len() < 2 {
        return;
    }
    let mut i = 0;
    while i + 1 < tokens.len() {
        if tokens[i].end == tokens[i + 1].start && tokens[i].scopes == tokens[i + 1].scopes {
            tokens[i].end = tokens[i + 1].end;
            tokens.remove(i + 1);
        } else {
            i += 1;
        }
    }
}

/// Tokenizes a single line using the provided grammar and mutable state.
///
/// This is a convenience wrapper around [`TextMateTokenizer::tokenize_line`].
pub fn tokenize_line(
    grammar: &TextMateGrammar,
    line: &str,
    state: &mut TokenizerState,
) -> Vec<TokenInfo> {
    let tokenizer = TextMateTokenizer::new(grammar);
    tokenizer.tokenize_line(line, state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_grammar() -> TextMateGrammar {
        TextMateGrammar {
            scope_name: "source.test".into(),
            file_types: vec!["test".into()],
            patterns: vec![
                Pattern::Match(MatchRule {
                    regex: r"//.*".into(),
                    scope: Some("comment.line".into()),
                    captures: HashMap::new(),
                }),
                Pattern::Match(MatchRule {
                    regex: r#""[^"]*""#.into(),
                    scope: Some("string.quoted.double".into()),
                    captures: HashMap::new(),
                }),
                Pattern::Match(MatchRule {
                    regex: r"\b(fn|let|if|else|return)\b".into(),
                    scope: Some("keyword.control".into()),
                    captures: HashMap::new(),
                }),
                Pattern::Match(MatchRule {
                    regex: r"\b\d+\b".into(),
                    scope: Some("constant.numeric".into()),
                    captures: HashMap::new(),
                }),
            ],
            repository: HashMap::new(),
        }
    }

    #[test]
    fn tokenize_keywords() {
        let grammar = make_simple_grammar();
        let mut state = TokenizerState::new();
        let tokens = tokenize_line(&grammar, "fn main", &mut state);

        let kw = tokens
            .iter()
            .find(|t| t.scopes.iter().any(|s| s.contains("keyword")));
        assert!(kw.is_some(), "should find a keyword token");
    }

    #[test]
    fn tokenize_comment() {
        let grammar = make_simple_grammar();
        let mut state = TokenizerState::new();
        let tokens = tokenize_line(&grammar, "// hello world", &mut state);

        assert!(!tokens.is_empty());
        let comment = tokens
            .iter()
            .find(|t| t.scopes.iter().any(|s| s.contains("comment")));
        assert!(comment.is_some(), "should find a comment token");
    }

    #[test]
    fn tokenize_string() {
        let grammar = make_simple_grammar();
        let mut state = TokenizerState::new();
        let tokens = tokenize_line(&grammar, r#"let x = "hello""#, &mut state);

        let string_tok = tokens
            .iter()
            .find(|t| t.scopes.iter().any(|s| s.contains("string")));
        assert!(string_tok.is_some(), "should find a string token");
    }

    #[test]
    fn tokenize_number() {
        let grammar = make_simple_grammar();
        let mut state = TokenizerState::new();
        let tokens = tokenize_line(&grammar, "let x = 42", &mut state);

        let num = tokens
            .iter()
            .find(|t| t.scopes.iter().any(|s| s.contains("numeric")));
        assert!(num.is_some(), "should find a numeric token");
    }

    #[test]
    fn empty_line_produces_no_tokens() {
        let grammar = make_simple_grammar();
        let mut state = TokenizerState::new();
        let tokens = tokenize_line(&grammar, "", &mut state);
        assert!(tokens.is_empty());
    }

    #[test]
    fn state_default() {
        let state = TokenizerState::default();
        assert!(state.rule_stack.is_empty());
    }

    #[test]
    fn token_info_fields() {
        let tok = TokenInfo {
            start: 0,
            end: 5,
            scopes: vec!["source.test".into(), "keyword.control".into()],
        };
        assert_eq!(tok.start, 0);
        assert_eq!(tok.end, 5);
        assert_eq!(tok.scopes.len(), 2);
    }

    #[test]
    fn from_json_basic() {
        let json = r#"{
            "scopeName": "source.example",
            "fileTypes": ["ex"],
            "patterns": [
                { "match": "\\bif\\b", "name": "keyword.control" }
            ],
            "repository": {}
        }"#;
        let grammar = TextMateGrammar::from_json(json).unwrap();
        assert_eq!(grammar.scope_name, "source.example");
        assert_eq!(grammar.file_types, vec!["ex"]);
        assert_eq!(grammar.patterns.len(), 1);
    }

    #[test]
    fn include_self_ref() {
        let grammar = TextMateGrammar {
            scope_name: "source.test".into(),
            file_types: vec![],
            patterns: vec![
                Pattern::Match(MatchRule {
                    regex: r"\bfn\b".into(),
                    scope: Some("keyword".into()),
                    captures: HashMap::new(),
                }),
                Pattern::Include(IncludeRef::SelfRef),
            ],
            repository: HashMap::new(),
        };
        let mut state = TokenizerState::new();
        let tokens = tokenize_line(&grammar, "fn", &mut state);
        assert!(!tokens.is_empty());
    }

    #[test]
    fn include_repository() {
        let mut repo = HashMap::new();
        repo.insert(
            "keywords".into(),
            RepositoryRule {
                patterns: vec![Pattern::Match(MatchRule {
                    regex: r"\blet\b".into(),
                    scope: Some("keyword".into()),
                    captures: HashMap::new(),
                })],
            },
        );
        let grammar = TextMateGrammar {
            scope_name: "source.test".into(),
            file_types: vec![],
            patterns: vec![Pattern::Include(IncludeRef::Repository("keywords".into()))],
            repository: repo,
        };
        let mut state = TokenizerState::new();
        let tokens = tokenize_line(&grammar, "let x = 1", &mut state);
        let kw = tokens
            .iter()
            .find(|t| t.scopes.iter().any(|s| s == "keyword"));
        assert!(kw.is_some());
    }

    #[test]
    fn merge_adjacent() {
        let mut tokens = vec![
            TokenInfo {
                start: 0,
                end: 3,
                scopes: vec!["a".into()],
            },
            TokenInfo {
                start: 3,
                end: 6,
                scopes: vec!["a".into()],
            },
            TokenInfo {
                start: 6,
                end: 9,
                scopes: vec!["b".into()],
            },
        ];
        merge_adjacent_tokens(&mut tokens);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].start, 0);
        assert_eq!(tokens[0].end, 6);
        assert_eq!(tokens[1].start, 6);
        assert_eq!(tokens[1].end, 9);
    }

    #[test]
    fn begin_end_rule_in_grammar() {
        let grammar = TextMateGrammar {
            scope_name: "source.test".into(),
            file_types: vec![],
            patterns: vec![Pattern::BeginEnd(BeginEndRule {
                begin: r#"""#.into(),
                end: r#"""#.into(),
                scope: Some("string.quoted.double".into()),
                begin_captures: HashMap::new(),
                end_captures: HashMap::new(),
                patterns: vec![],
            })],
            repository: HashMap::new(),
        };
        let mut state = TokenizerState::new();
        let tokens = tokenize_line(&grammar, r#""hello""#, &mut state);
        assert!(!tokens.is_empty());
    }

    #[test]
    fn parse_include_variants() {
        assert!(matches!(parse_include("$self"), IncludeRef::SelfRef));
        assert!(matches!(parse_include("$base"), IncludeRef::BaseRef));
        assert!(matches!(
            parse_include("#keywords"),
            IncludeRef::Repository(ref s) if s == "keywords"
        ));
        assert!(matches!(
            parse_include("source.other"),
            IncludeRef::External(ref s) if s == "source.other"
        ));
    }

    #[test]
    fn unsupported_format_error() {
        let result = TextMateGrammar::from_file(Path::new("test.xyz"));
        assert!(result.is_err());
    }
}
