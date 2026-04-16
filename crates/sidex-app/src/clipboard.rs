//! Clipboard operations using the `arboard` crate.

use anyhow::{Context, Result};

/// Copies text to the system clipboard.
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new().context("failed to init clipboard")?;
    clipboard
        .set_text(text)
        .context("failed to set clipboard text")?;
    Ok(())
}

/// Reads text from the system clipboard. Returns `None` if the clipboard
/// is empty or does not contain text.
pub fn paste_from_clipboard() -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    clipboard.get_text().ok()
}

/// Cuts text: copies to clipboard (the caller is responsible for deleting
/// the source text from the document).
pub fn cut_to_clipboard(text: &str) -> Result<()> {
    copy_to_clipboard(text)
}
