//! Context keys for evaluating keybinding "when" clauses.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A value stored in the context key map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContextValue {
    Bool(bool),
    String(String),
}

impl ContextValue {
    /// Coerce to bool: `Bool(b)` → `b`, non-empty `String` → `true`.
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::String(s) => !s.is_empty(),
        }
    }
}

/// A key-value store of contextual state used to evaluate "when" clauses
/// on keybindings (e.g. `editorTextFocus && !editorReadonly`).
#[derive(Clone, Debug, Default)]
pub struct ContextKeys {
    map: HashMap<String, ContextValue>,
}

impl ContextKeys {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a context key.
    pub fn set(&mut self, key: impl Into<String>, value: ContextValue) {
        self.map.insert(key.into(), value);
    }

    /// Set a boolean context key (convenience).
    pub fn set_bool(&mut self, key: impl Into<String>, value: bool) {
        self.set(key, ContextValue::Bool(value));
    }

    /// Set a string context key (convenience).
    pub fn set_string(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.set(key, ContextValue::String(value.into()));
    }

    /// Get a context value by key.
    pub fn get(&self, key: &str) -> Option<&ContextValue> {
        self.map.get(key)
    }

    /// Get a context key as a boolean, defaulting to `false` if unset.
    pub fn is_true(&self, key: &str) -> bool {
        self.get(key).is_some_and(ContextValue::as_bool)
    }

    /// Remove a context key.
    pub fn remove(&mut self, key: &str) {
        self.map.remove(key);
    }
}

// ── Expression evaluator ─────────────────────────────────────────────────────

/// Evaluate a "when" clause expression against a set of context keys.
///
/// Supported operators: `==`, `!=`, `&&`, `||`, `!`, parentheses.
/// Identifiers are looked up in `context`; unknown keys evaluate to `false`.
///
/// Examples:
/// - `"editorTextFocus"`
/// - `"editorTextFocus && !editorReadonly"`
/// - `"resourceScheme == 'file'"`
pub fn evaluate(expression: &str, context: &ContextKeys) -> bool {
    let tokens = tokenize(expression);
    let mut parser = Parser::new(&tokens);
    parser.parse_or(context)
}

// ── Tokenizer ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    StringLit(String),
    And,
    Or,
    Not,
    Eq,
    Neq,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        match chars[i] {
            ' ' | '\t' | '\n' | '\r' => i += 1,
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '!' if i + 1 < len && chars[i + 1] == '=' => {
                tokens.push(Token::Neq);
                i += 2;
            }
            '!' => {
                tokens.push(Token::Not);
                i += 1;
            }
            '=' if i + 1 < len && chars[i + 1] == '=' => {
                tokens.push(Token::Eq);
                i += 2;
            }
            '&' if i + 1 < len && chars[i + 1] == '&' => {
                tokens.push(Token::And);
                i += 2;
            }
            '|' if i + 1 < len && chars[i + 1] == '|' => {
                tokens.push(Token::Or);
                i += 2;
            }
            '\'' | '"' => {
                let quote = chars[i];
                i += 1;
                let start = i;
                while i < len && chars[i] != quote {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::StringLit(s));
                if i < len {
                    i += 1;
                }
            }
            _ => {
                let start = i;
                while i < len && !matches!(chars[i], ' ' | '\t' | '(' | ')' | '!' | '=' | '&' | '|' | '\'' | '"') {
                    i += 1;
                }
                let ident: String = chars[start..i].iter().collect();
                if !ident.is_empty() {
                    tokens.push(Token::Ident(ident));
                }
            }
        }
    }

    tokens
}

// ── Recursive-descent parser ─────────────────────────────────────────────────

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn parse_or(&mut self, ctx: &ContextKeys) -> bool {
        let mut result = self.parse_and(ctx);
        while self.peek() == Some(&Token::Or) {
            self.advance();
            let rhs = self.parse_and(ctx);
            result = result || rhs;
        }
        result
    }

    fn parse_and(&mut self, ctx: &ContextKeys) -> bool {
        let mut result = self.parse_unary(ctx);
        while self.peek() == Some(&Token::And) {
            self.advance();
            let rhs = self.parse_unary(ctx);
            result = result && rhs;
        }
        result
    }

    fn parse_unary(&mut self, ctx: &ContextKeys) -> bool {
        if self.peek() == Some(&Token::Not) {
            self.advance();
            return !self.parse_unary(ctx);
        }
        self.parse_primary(ctx)
    }

    fn parse_primary(&mut self, ctx: &ContextKeys) -> bool {
        if self.peek() == Some(&Token::LParen) {
            self.advance();
            let result = self.parse_or(ctx);
            if self.peek() == Some(&Token::RParen) {
                self.advance();
            }
            return result;
        }

        let ident = match self.advance() {
            Some(Token::Ident(s)) => s.clone(),
            Some(Token::StringLit(s)) => return !s.is_empty(),
            _ => return false,
        };

        match self.peek() {
            Some(Token::Eq) => {
                self.advance();
                let rhs = self.parse_value();
                match ctx.get(&ident) {
                    Some(ContextValue::String(s)) => *s == rhs,
                    Some(ContextValue::Bool(b)) => {
                        (rhs == "true" && *b) || (rhs == "false" && !*b)
                    }
                    None => rhs == "false",
                }
            }
            Some(Token::Neq) => {
                self.advance();
                let rhs = self.parse_value();
                match ctx.get(&ident) {
                    Some(ContextValue::String(s)) => *s != rhs,
                    Some(ContextValue::Bool(b)) => {
                        !((rhs == "true" && *b) || (rhs == "false" && !*b))
                    }
                    None => rhs != "false",
                }
            }
            _ => ctx.is_true(&ident),
        }
    }

    fn parse_value(&mut self) -> String {
        match self.advance() {
            Some(Token::Ident(s) | Token::StringLit(s)) => s.clone(),
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_with(pairs: &[(&str, bool)]) -> ContextKeys {
        let mut ctx = ContextKeys::new();
        for &(k, v) in pairs {
            ctx.set_bool(k, v);
        }
        ctx
    }

    #[test]
    fn simple_true() {
        let ctx = ctx_with(&[("editorTextFocus", true)]);
        assert!(evaluate("editorTextFocus", &ctx));
    }

    #[test]
    fn simple_false() {
        let ctx = ctx_with(&[("editorTextFocus", false)]);
        assert!(!evaluate("editorTextFocus", &ctx));
    }

    #[test]
    fn missing_key_is_false() {
        let ctx = ContextKeys::new();
        assert!(!evaluate("editorTextFocus", &ctx));
    }

    #[test]
    fn negation() {
        let ctx = ctx_with(&[("editorReadonly", false)]);
        assert!(evaluate("!editorReadonly", &ctx));
    }

    #[test]
    fn and_expression() {
        let ctx = ctx_with(&[("editorTextFocus", true), ("editorReadonly", false)]);
        assert!(evaluate("editorTextFocus && !editorReadonly", &ctx));
    }

    #[test]
    fn or_expression() {
        let ctx = ctx_with(&[("a", false), ("b", true)]);
        assert!(evaluate("a || b", &ctx));
    }

    #[test]
    fn equality() {
        let mut ctx = ContextKeys::new();
        ctx.set_string("resourceScheme", "file");
        assert!(evaluate("resourceScheme == 'file'", &ctx));
        assert!(!evaluate("resourceScheme == 'untitled'", &ctx));
    }

    #[test]
    fn inequality() {
        let mut ctx = ContextKeys::new();
        ctx.set_string("resourceScheme", "file");
        assert!(evaluate("resourceScheme != 'untitled'", &ctx));
    }

    #[test]
    fn parentheses() {
        let ctx = ctx_with(&[("a", true), ("b", false), ("c", true)]);
        assert!(evaluate("a && (b || c)", &ctx));
        assert!(!evaluate("(a && b) || !c", &ctx));
    }

    #[test]
    fn complex_expression() {
        let mut ctx = ContextKeys::new();
        ctx.set_bool("editorTextFocus", true);
        ctx.set_bool("editorReadonly", false);
        ctx.set_string("resourceScheme", "file");
        assert!(evaluate(
            "editorTextFocus && !editorReadonly && resourceScheme == 'file'",
            &ctx
        ));
    }

    #[test]
    fn context_value_as_bool() {
        assert!(ContextValue::Bool(true).as_bool());
        assert!(!ContextValue::Bool(false).as_bool());
        assert!(ContextValue::String("hello".to_owned()).as_bool());
        assert!(!ContextValue::String(String::new()).as_bool());
    }
}
