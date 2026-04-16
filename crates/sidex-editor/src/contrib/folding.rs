//! Code folding model — mirrors VS Code's `FoldingModel` +
//! `FoldingRanges` + indent/syntax/marker providers.
//!
//! Tracks which regions of a document are foldable and whether each region is
//! currently collapsed.  Folding ranges can originate from indentation
//! analysis, tree-sitter (language), or explicit markers (`#region`/`#endregion`).

use sidex_text::Buffer;

/// The source that produced a folding range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldSource {
    /// Computed from indentation levels.
    Indentation,
    /// Provided by a language provider (tree-sitter / LSP).
    Language,
    /// Explicit markers (#region / #endregion, // region, etc.).
    Marker,
    /// Manually toggled by the user.
    Manual,
}

/// The semantic kind of a folding range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldKind {
    /// A general region (function body, object literal, etc.).
    Region,
    /// An import block.
    Imports,
    /// A comment block.
    Comment,
}

/// A single foldable region in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldingRegion {
    /// First line of the region (zero-based).
    pub start_line: u32,
    /// Last line of the region (zero-based, inclusive).
    pub end_line: u32,
    /// Whether this region is currently collapsed.
    pub is_collapsed: bool,
    /// How this region was detected.
    pub source: FoldSource,
    /// Semantic kind (if known).
    pub kind: Option<FoldKind>,
}

impl FoldingRegion {
    #[must_use]
    pub fn line_count(&self) -> u32 {
        self.end_line.saturating_sub(self.start_line) + 1
    }

    #[must_use]
    pub fn contains_line(&self, line: u32) -> bool {
        line >= self.start_line && line <= self.end_line
    }
}

/// The complete folding state for a document.
#[derive(Debug, Clone, Default)]
pub struct FoldingModel {
    /// All known folding regions, sorted by `start_line`.
    regions: Vec<FoldingRegion>,
}

impl FoldingModel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the regions with a new set (e.g. after a re-parse).  Preserves
    /// the collapsed state for regions whose start/end lines still match.
    pub fn update_regions(&mut self, mut new_regions: Vec<FoldingRegion>) {
        for nr in &mut new_regions {
            if let Some(existing) = self
                .regions
                .iter()
                .find(|r| r.start_line == nr.start_line && r.end_line == nr.end_line)
            {
                nr.is_collapsed = existing.is_collapsed;
            }
        }
        new_regions.sort_by_key(|r| r.start_line);
        self.regions = new_regions;
    }

    /// Returns a reference to all regions.
    #[must_use]
    pub fn regions(&self) -> &[FoldingRegion] {
        &self.regions
    }

    /// Returns a mutable reference to all regions.
    pub fn regions_mut(&mut self) -> &mut [FoldingRegion] {
        &mut self.regions
    }

    /// Returns the region that starts on the given `line`, if any.
    #[must_use]
    pub fn region_at_line(&self, line: u32) -> Option<&FoldingRegion> {
        self.regions.iter().find(|r| r.start_line == line)
    }

    /// Toggles the collapsed state of the region that starts on `line`.
    /// Returns `true` if a region was toggled.
    pub fn toggle_fold(&mut self, line: u32) -> bool {
        if let Some(r) = self.regions.iter_mut().find(|r| r.start_line == line) {
            r.is_collapsed = !r.is_collapsed;
            true
        } else {
            false
        }
    }

    /// Collapses all regions.
    pub fn fold_all(&mut self) {
        for r in &mut self.regions {
            r.is_collapsed = true;
        }
    }

    /// Expands all regions.
    pub fn unfold_all(&mut self) {
        for r in &mut self.regions {
            r.is_collapsed = false;
        }
    }

    /// Collapses all regions at the given nesting level and deeper.
    /// Level 1 = top-level regions, level 2 = nested inside level 1, etc.
    pub fn fold_level(&mut self, level: u32) {
        let levels = self.compute_nesting_levels();
        for (i, r) in self.regions.iter_mut().enumerate() {
            r.is_collapsed = levels[i] >= level;
        }
    }

    /// Returns the set of lines that are hidden (inside a collapsed region,
    /// excluding the start line of each region).
    #[must_use]
    pub fn hidden_lines(&self) -> Vec<u32> {
        let mut hidden = Vec::new();
        for r in &self.regions {
            if r.is_collapsed {
                for line in (r.start_line + 1)..=r.end_line {
                    hidden.push(line);
                }
            }
        }
        hidden.sort_unstable();
        hidden.dedup();
        hidden
    }

    /// Returns `true` if `line` should be hidden by a collapsed fold.
    #[must_use]
    pub fn is_line_hidden(&self, line: u32) -> bool {
        self.regions
            .iter()
            .any(|r| r.is_collapsed && line > r.start_line && line <= r.end_line)
    }

    /// Computes folding regions from indentation levels.
    pub fn compute_from_indentation(buffer: &Buffer, tab_size: u32) -> Vec<FoldingRegion> {
        let line_count = buffer.len_lines();
        if line_count == 0 {
            return Vec::new();
        }

        let indents: Vec<Option<u32>> = (0..line_count)
            .map(|i| {
                let content = buffer.line_content(i);
                let trimmed = content.trim_start();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(Self::visible_indent(&content, tab_size))
                }
            })
            .collect();

        let mut regions = Vec::new();
        let mut stack: Vec<(u32, u32)> = Vec::new(); // (indent_level, start_line)

        for (i, indent_opt) in indents.iter().enumerate() {
            let line = i as u32;
            if let Some(&indent) = indent_opt.as_ref() {
                while let Some(&(top_indent, top_start)) = stack.last() {
                    if indent <= top_indent {
                        stack.pop();
                        if line.saturating_sub(1) > top_start {
                            regions.push(FoldingRegion {
                                start_line: top_start,
                                end_line: line.saturating_sub(1),
                                is_collapsed: false,
                                source: FoldSource::Indentation,
                                kind: None,
                            });
                        }
                    } else {
                        break;
                    }
                }
                stack.push((indent, line));
            }
        }

        let last_line = (line_count - 1) as u32;
        while let Some((_, start)) = stack.pop() {
            if last_line > start {
                regions.push(FoldingRegion {
                    start_line: start,
                    end_line: last_line,
                    is_collapsed: false,
                    source: FoldSource::Indentation,
                    kind: None,
                });
            }
        }

        regions.sort_by_key(|r| r.start_line);
        regions
    }

    /// Detects `#region` / `#endregion` style markers.
    pub fn compute_from_markers(
        buffer: &Buffer,
        start_marker: &str,
        end_marker: &str,
    ) -> Vec<FoldingRegion> {
        let mut regions = Vec::new();
        let mut stack: Vec<u32> = Vec::new();

        for i in 0..buffer.len_lines() {
            let content = buffer.line_content(i);
            let trimmed = content.trim();
            if trimmed.contains(start_marker) {
                stack.push(i as u32);
            } else if trimmed.contains(end_marker) {
                if let Some(start) = stack.pop() {
                    regions.push(FoldingRegion {
                        start_line: start,
                        end_line: i as u32,
                        is_collapsed: false,
                        source: FoldSource::Marker,
                        kind: Some(FoldKind::Region),
                    });
                }
            }
        }

        regions.sort_by_key(|r| r.start_line);
        regions
    }

    // ── Private helpers ─────────────────────────────────────────────────

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

    fn compute_nesting_levels(&self) -> Vec<u32> {
        let mut levels = vec![0u32; self.regions.len()];
        for (i, region) in self.regions.iter().enumerate() {
            let mut depth = 1u32;
            for parent in &self.regions[..i] {
                if parent.start_line < region.start_line
                    && parent.end_line >= region.end_line
                {
                    depth += 1;
                }
            }
            levels[i] = depth;
        }
        levels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str) -> Buffer {
        Buffer::from_str(text)
    }

    #[test]
    fn toggle_fold() {
        let mut model = FoldingModel::new();
        model.regions = vec![FoldingRegion {
            start_line: 0,
            end_line: 5,
            is_collapsed: false,
            source: FoldSource::Language,
            kind: None,
        }];
        assert!(model.toggle_fold(0));
        assert!(model.regions[0].is_collapsed);
        assert!(model.toggle_fold(0));
        assert!(!model.regions[0].is_collapsed);
    }

    #[test]
    fn hidden_lines() {
        let mut model = FoldingModel::new();
        model.regions = vec![FoldingRegion {
            start_line: 2,
            end_line: 5,
            is_collapsed: true,
            source: FoldSource::Language,
            kind: None,
        }];
        let hidden = model.hidden_lines();
        assert_eq!(hidden, vec![3, 4, 5]);
        assert!(!model.is_line_hidden(2));
        assert!(model.is_line_hidden(3));
    }

    #[test]
    fn indentation_folding() {
        let text = "fn main() {\n    let x = 1;\n    let y = 2;\n}\n";
        let buffer = buf(text);
        let regions = FoldingModel::compute_from_indentation(&buffer, 4);
        assert!(!regions.is_empty());
        assert_eq!(regions[0].start_line, 0);
    }

    #[test]
    fn marker_folding() {
        let text = "// #region Foo\ncode\nmore\n// #endregion\n";
        let buffer = buf(text);
        let regions = FoldingModel::compute_from_markers(&buffer, "#region", "#endregion");
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].start_line, 0);
        assert_eq!(regions[0].end_line, 3);
    }
}
