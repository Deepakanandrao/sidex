//! Indentation guide lines — mirrors VS Code's indent-guides view part.
//!
//! Computes the vertical indentation guide lines to render in the editor
//! gutter/text area.

use sidex_text::Buffer;

/// A single indentation guide line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndentGuide {
    /// The document line this guide is on (zero-based).
    pub line: u32,
    /// The column (character offset) of the guide (zero-based).
    pub column: u32,
    /// Whether this guide is the "active" one (the scope containing the cursor).
    pub is_active: bool,
    /// Nesting level (1 = first indent, 2 = second, etc.).
    pub level: u32,
}

/// Computes indent guides for the visible viewport.
///
/// `active_line` is the cursor line; guides at the cursor's indent scope are
/// marked as active.
#[must_use]
pub fn compute_indent_guides(
    buffer: &Buffer,
    first_line: u32,
    last_line: u32,
    tab_size: u32,
    active_line: Option<u32>,
) -> Vec<IndentGuide> {
    let mut guides = Vec::new();
    let line_count = buffer.len_lines() as u32;

    let active_indent_col = active_line.map(|l| {
        if (l as usize) < buffer.len_lines() {
            let content = buffer.line_content(l as usize);
            visible_indent(&content, tab_size)
        } else {
            0
        }
    });

    for line_idx in first_line..=last_line.min(line_count.saturating_sub(1)) {
        let content = buffer.line_content(line_idx as usize);
        let indent = visible_indent(&content, tab_size);

        let num_guides = indent / tab_size;
        for g in 0..num_guides {
            let col = g * tab_size;
            let level = g + 1;
            let is_active = active_indent_col
                .is_some_and(|ac| col < ac && active_line.is_some());
            guides.push(IndentGuide {
                line: line_idx,
                column: col,
                is_active,
                level,
            });
        }
    }

    guides
}

/// Returns the visible indentation width of a line, expanding tabs.
fn visible_indent(line: &str, tab_size: u32) -> u32 {
    let mut indent = 0u32;
    for ch in line.chars() {
        match ch {
            ' ' => indent += 1,
            '\t' => indent += tab_size - (indent % tab_size),
            _ => break,
        }
    }
    indent
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str) -> Buffer {
        Buffer::from_str(text)
    }

    #[test]
    fn basic_indent_guides() {
        let buffer = buf("fn main() {\n    let x = 1;\n}");
        let guides = compute_indent_guides(&buffer, 0, 2, 4, None);
        // Line 1 ("    let x = 1;") should have one guide at column 0
        let line1_guides: Vec<_> = guides.iter().filter(|g| g.line == 1).collect();
        assert_eq!(line1_guides.len(), 1);
        assert_eq!(line1_guides[0].column, 0);
    }

    #[test]
    fn nested_indent() {
        let buffer = buf("a\n    b\n        c\n    d\ne");
        let guides = compute_indent_guides(&buffer, 0, 4, 4, None);
        let line2_guides: Vec<_> = guides.iter().filter(|g| g.line == 2).collect();
        assert_eq!(line2_guides.len(), 2); // two levels of indent
    }
}
