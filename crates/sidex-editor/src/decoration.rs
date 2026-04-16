//! Text decoration management — inline decorations, glyph margin markers,
//! overview ruler indicators, and whole-line highlights.

use serde::{Deserialize, Serialize};

use sidex_text::Range;

/// Unique identifier for a set of decorations added together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DecorationSetId(u64);

/// An RGBA color for decorations.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Creates a new color from RGBA components (0.0–1.0).
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const YELLOW: Self = Self::new(1.0, 1.0, 0.0, 1.0);
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
}

impl Eq for Color {}

/// Visual options for a single decoration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DecorationOptions {
    /// CSS class name to apply to the decorated range.
    pub class_name: Option<String>,
    /// Foreground color override.
    pub color: Option<Color>,
    /// Background color override.
    pub background_color: Option<Color>,
    /// Whether to draw an outline box around the range.
    pub outline: bool,
    /// Whether the decoration applies to the entire line, not just the range.
    pub is_whole_line: bool,
    /// CSS class name for the glyph margin (line-number gutter area).
    pub glyph_margin_class: Option<String>,
    /// Color to show in the overview ruler (minimap scrollbar).
    pub overview_ruler_color: Option<Color>,
}

impl DecorationOptions {
    /// Builder: set class name.
    #[must_use]
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class_name = Some(class.into());
        self
    }

    /// Builder: set foreground color.
    #[must_use]
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Builder: set background color.
    #[must_use]
    pub fn with_background(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Builder: enable outline.
    #[must_use]
    pub fn with_outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Builder: make whole-line.
    #[must_use]
    pub fn whole_line(mut self) -> Self {
        self.is_whole_line = true;
        self
    }
}

/// A single text decoration applied to a range in the document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decoration {
    /// The document range this decoration covers.
    pub range: Range,
    /// Visual options.
    pub options: DecorationOptions,
}

impl Decoration {
    /// Creates a new decoration.
    pub fn new(range: Range, options: DecorationOptions) -> Self {
        Self { range, options }
    }
}

/// A stored set of decorations with its ID.
#[derive(Debug, Clone)]
struct DecorationSet {
    id: DecorationSetId,
    decorations: Vec<Decoration>,
}

/// Manages all decoration sets for a document.
#[derive(Debug, Clone)]
pub struct DecorationCollection {
    sets: Vec<DecorationSet>,
    next_id: u64,
}

impl DecorationCollection {
    /// Creates a new, empty collection.
    pub fn new() -> Self {
        Self {
            sets: Vec::new(),
            next_id: 1,
        }
    }

    /// Adds a batch of decorations and returns an ID that can be used to
    /// remove them later.
    pub fn add(&mut self, decorations: Vec<Decoration>) -> DecorationSetId {
        let id = DecorationSetId(self.next_id);
        self.next_id += 1;
        self.sets.push(DecorationSet { id, decorations });
        id
    }

    /// Removes a previously added decoration set by ID.
    pub fn remove(&mut self, id: DecorationSetId) {
        self.sets.retain(|s| s.id != id);
    }

    /// Returns all decorations whose range intersects with the given range.
    pub fn decorations_in_range(&self, range: Range) -> Vec<&Decoration> {
        self.sets
            .iter()
            .flat_map(|s| s.decorations.iter())
            .filter(|d| d.range.intersects(&range))
            .collect()
    }

    /// Returns all decorations across all sets.
    pub fn all_decorations(&self) -> Vec<&Decoration> {
        self.sets
            .iter()
            .flat_map(|s| s.decorations.iter())
            .collect()
    }

    /// Returns the number of decoration sets.
    pub fn set_count(&self) -> usize {
        self.sets.len()
    }

    /// Returns the total number of individual decorations.
    pub fn decoration_count(&self) -> usize {
        self.sets.iter().map(|s| s.decorations.len()).sum()
    }

    /// Removes all decoration sets.
    pub fn clear(&mut self) {
        self.sets.clear();
    }
}

impl Default for DecorationCollection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use sidex_text::Position;

    use super::*;

    fn range(sl: u32, sc: u32, el: u32, ec: u32) -> Range {
        Range::new(Position::new(sl, sc), Position::new(el, ec))
    }

    #[test]
    fn add_and_query() {
        let mut coll = DecorationCollection::new();
        let decs = vec![Decoration::new(
            range(0, 0, 0, 5),
            DecorationOptions::default().with_class("error"),
        )];
        let id = coll.add(decs);
        assert_eq!(coll.set_count(), 1);
        assert_eq!(coll.decoration_count(), 1);

        let found = coll.decorations_in_range(range(0, 0, 0, 10));
        assert_eq!(found.len(), 1);

        coll.remove(id);
        assert_eq!(coll.set_count(), 0);
    }

    #[test]
    fn query_range_filtering() {
        let mut coll = DecorationCollection::new();
        coll.add(vec![
            Decoration::new(range(0, 0, 0, 5), DecorationOptions::default()),
            Decoration::new(range(5, 0, 5, 10), DecorationOptions::default()),
            Decoration::new(range(10, 0, 10, 5), DecorationOptions::default()),
        ]);

        let found = coll.decorations_in_range(range(4, 0, 6, 0));
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn multiple_sets() {
        let mut coll = DecorationCollection::new();
        let id1 = coll.add(vec![Decoration::new(
            range(0, 0, 0, 5),
            DecorationOptions::default(),
        )]);
        let id2 = coll.add(vec![Decoration::new(
            range(1, 0, 1, 5),
            DecorationOptions::default(),
        )]);
        assert_eq!(coll.set_count(), 2);
        assert_eq!(coll.decoration_count(), 2);

        coll.remove(id1);
        assert_eq!(coll.set_count(), 1);
        assert_eq!(coll.all_decorations().len(), 1);

        coll.remove(id2);
        assert_eq!(coll.set_count(), 0);
    }

    #[test]
    fn clear_all() {
        let mut coll = DecorationCollection::new();
        coll.add(vec![Decoration::new(
            range(0, 0, 0, 5),
            DecorationOptions::default(),
        )]);
        coll.add(vec![Decoration::new(
            range(1, 0, 1, 5),
            DecorationOptions::default(),
        )]);
        coll.clear();
        assert_eq!(coll.set_count(), 0);
        assert_eq!(coll.decoration_count(), 0);
    }

    #[test]
    fn decoration_options_builder() {
        let opts = DecorationOptions::default()
            .with_class("highlight")
            .with_color(Color::RED)
            .with_background(Color::YELLOW)
            .with_outline()
            .whole_line();

        assert_eq!(opts.class_name.as_deref(), Some("highlight"));
        assert_eq!(opts.color, Some(Color::RED));
        assert_eq!(opts.background_color, Some(Color::YELLOW));
        assert!(opts.outline);
        assert!(opts.is_whole_line);
    }

    #[test]
    fn color_constants() {
        assert_eq!(Color::RED.r, 1.0);
        assert_eq!(Color::GREEN.g, 1.0);
        assert_eq!(Color::BLUE.b, 1.0);
        assert_eq!(Color::TRANSPARENT.a, 0.0);
    }

    #[test]
    fn remove_nonexistent_id_is_noop() {
        let mut coll = DecorationCollection::new();
        coll.add(vec![Decoration::new(
            range(0, 0, 0, 5),
            DecorationOptions::default(),
        )]);
        coll.remove(DecorationSetId(999));
        assert_eq!(coll.set_count(), 1);
    }

    #[test]
    fn no_intersecting_decorations() {
        let mut coll = DecorationCollection::new();
        coll.add(vec![Decoration::new(
            range(10, 0, 10, 5),
            DecorationOptions::default(),
        )]);
        let found = coll.decorations_in_range(range(0, 0, 0, 100));
        assert!(found.is_empty());
    }
}
