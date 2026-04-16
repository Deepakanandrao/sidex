//! Snippet controller — mirrors VS Code's `SnippetController2`.
//!
//! Manages the active snippet session's tabstop navigation, exposing it as a
//! contribution-level concern separate from the core snippet engine.

use crate::document::Document;
use crate::snippet::SnippetSession;

/// The contribution-level wrapper around an active snippet session.
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct SnippetControllerState {
    /// The active snippet session, if any.
    pub session: Option<SnippetSession>,
    /// Whether the snippet controller is "locked" (nested snippet in progress).
    pub is_nested: bool,
}


impl SnippetControllerState {
    /// Returns `true` if a snippet session is active and not finished.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|s| !s.finished)
    }

    /// Inserts a snippet and starts a new session.  If a session is already
    /// active, it becomes a nested session.
    pub fn insert_snippet(&mut self, document: &mut Document, template: &str) {
        if self.is_active() {
            self.is_nested = true;
        }
        let session = SnippetSession::start(document, template);
        self.session = Some(session);
    }

    /// Advances to the next tabstop.
    pub fn next_tabstop(&mut self, document: &mut Document) {
        if let Some(session) = self.session.as_mut() {
            session.next_tabstop(document);
            if session.finished {
                self.finish();
            }
        }
    }

    /// Moves to the previous tabstop.
    pub fn prev_tabstop(&mut self, document: &mut Document) {
        if let Some(session) = self.session.as_mut() {
            session.prev_tabstop(document);
        }
    }

    /// Cancels the active snippet session.
    pub fn cancel(&mut self) {
        self.session = None;
        self.is_nested = false;
    }

    /// Finishes the session (called when the last tabstop is reached).
    fn finish(&mut self) {
        self.session = None;
        self.is_nested = false;
    }
}
