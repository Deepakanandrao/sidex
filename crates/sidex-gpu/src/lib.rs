//! GPU-accelerated rendering for the `SideX` editor.
//!
//! This crate provides the rendering layer built on top of [`wgpu`]. It
//! contains:
//!
//! - [`Scene`] — scene graph with draw call batching and z-ordered layer
//!   dispatch, modeled after Zed's `gpui::scene`.
//! - [`GpuRenderer`] — core wgpu device, queue, surface management, and
//!   scene-based rendering with camera transform.
//! - [`TextLayoutSystem`] — text layout system using `cosmic-text` with
//!   line height, letter spacing, tab stops, word wrap, bidi, and caching.
//! - [`TextAtlas`] — dual-atlas glyph texture management (mask R8 + color
//!   Rgba8) with LRU eviction, subpixel positioning, and atlas compaction.
//! - [`TextRenderer`] — batched text drawing via instanced quads.
//! - [`RectRenderer`] — batched rectangle / shape drawing.
//! - [`Color`] — simple RGBA color type with conversions.
//! - [`CursorRenderer`] — blinking cursor with smooth animation.
//! - [`SelectionRenderer`] — selection backgrounds, highlights, bracket pairs.
//! - [`LineRenderer`] — full line rendering with decorations.
//! - [`GutterRenderer`] — line numbers, folds, breakpoints, git/diagnostic indicators.
//! - [`MinimapRenderer`] — scaled-down document overview.
//! - [`ScrollbarRenderer`] — scrollbars with overview ruler and smooth scrolling.
//! - [`EditorView`] — compositor that assembles all renderers into one frame.
//! - Vertex types and shader pipelines (text, subpixel text, rect, shadow, underline).

pub mod color;
pub mod cursor_renderer;
pub mod editor_view;
pub mod gutter;
pub mod line_renderer;
pub mod minimap;
pub mod pipeline;
pub mod rect_renderer;
pub mod renderer;
pub mod scene;
pub mod scroll;
pub mod selection_renderer;
pub mod text_atlas;
pub mod text_layout;
pub mod text_renderer;
pub mod vertex;

pub use color::Color;
pub use cursor_renderer::{CursorAnimConfig, CursorPosition, CursorRenderer, CursorStyle};
pub use editor_view::{DocumentSnapshot, EditorConfig, EditorView, FrameInput, HighlightResult};
pub use gutter::{
    Breakpoint, FoldMarker, FoldState, GutterConfig, GutterDiagnostic,
    GutterDiagnosticSeverity, GutterDiffKind, GutterDiffMark, GutterRenderer,
};
pub use line_renderer::{
    CodeLens, InlayHint, IndentGuide, LineRenderConfig, LineRenderer, StyledLine, StyledSpan,
    StickyHeader, TextStyle, Viewport, WhitespaceRender, WrapIndicator,
};
pub use minimap::{
    DiagnosticMark, DiagnosticSeverity, GitChange, GitChangeKind, LineRange, MinimapClickResult,
    MinimapConfig, MinimapRenderer, MinimapViewport,
};
pub use pipeline::{
    create_rect_pipeline, create_shadow_pipeline, create_subpixel_text_pipeline,
    create_text_pipeline, create_underline_pipeline,
};
pub use rect_renderer::RectRenderer;
pub use renderer::{FrameContext, GpuError, GpuRenderer};
pub use scene::{
    ContentMask, Layer, MonochromeSprite, PolychromeSprite, Quad, Scene, Shadow, SubpixelSprite,
    Underline,
};
pub use scroll::{
    OverviewMarkKind, OverviewRulerMark, ScrollbarConfig, ScrollbarRenderer, SmoothScrollAxis,
};
pub use selection_renderer::{
    BracketHighlight, HighlightRect, SelectionRect, SelectionRenderConfig, SelectionRenderer,
};
pub use text_atlas::{
    AtlasKind, ExtendedCacheKey, FontVariant, GlyphInfo, SubpixelBin, TextAtlas,
    SUBPIXEL_BINS_X, SUBPIXEL_BINS_Y,
};
pub use text_layout::{
    FontConfig, FontStyle, FontWeight, LineHeightMode, LineLayout, ShapedGlyph, ShapedRun,
    StyledTextRun, TextLayoutConfig, TextLayoutSystem, WrapBoundary, WrapMode,
};
pub use text_renderer::{TextDrawContext, TextRenderer};
pub use vertex::{RectVertex, TextVertex};
