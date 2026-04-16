//! Built-in default themes ported from VS Code.
//!
//! Provides four const-constructable themes: Default Dark Modern,
//! Default Light Modern, High Contrast, and High Contrast Light.

use crate::color::Color;
use crate::theme::{Theme, ThemeKind};
use crate::token_color::{FontStyle, TokenColorRule};
use crate::workbench_colors::WorkbenchColors;

/// "Default Dark Modern" — the VS Code default dark theme.
pub fn dark_modern() -> Theme {
    Theme {
        name: "Default Dark Modern".to_owned(),
        kind: ThemeKind::Dark,
        token_colors: dark_modern_tokens(),
        workbench_colors: WorkbenchColors::default_dark(),
    }
}

/// "Default Light Modern" — the VS Code default light theme.
pub fn light_modern() -> Theme {
    Theme {
        name: "Default Light Modern".to_owned(),
        kind: ThemeKind::Light,
        token_colors: light_modern_tokens(),
        workbench_colors: WorkbenchColors::default_light(),
    }
}

/// "Default High Contrast" — dark high-contrast theme.
pub fn hc_black() -> Theme {
    Theme {
        name: "Default High Contrast".to_owned(),
        kind: ThemeKind::HighContrast,
        token_colors: hc_black_tokens(),
        workbench_colors: hc_black_colors(),
    }
}

/// "Default High Contrast Light" — light high-contrast theme.
pub fn hc_light() -> Theme {
    Theme {
        name: "Default High Contrast Light".to_owned(),
        kind: ThemeKind::HighContrastLight,
        token_colors: hc_light_tokens(),
        workbench_colors: hc_light_colors(),
    }
}

fn tok(scope: &str, fg: &str) -> TokenColorRule {
    TokenColorRule {
        name: None,
        scope: vec![scope.to_owned()],
        foreground: Color::from_hex(fg).ok(),
        background: None,
        font_style: FontStyle::NONE,
    }
}

fn tok_multi(scopes: &[&str], fg: &str) -> TokenColorRule {
    TokenColorRule {
        name: None,
        scope: scopes.iter().map(|s| (*s).to_owned()).collect(),
        foreground: Color::from_hex(fg).ok(),
        background: None,
        font_style: FontStyle::NONE,
    }
}

fn tok_styled(scope: &str, fg: &str, style: FontStyle) -> TokenColorRule {
    TokenColorRule {
        name: None,
        scope: vec![scope.to_owned()],
        foreground: Color::from_hex(fg).ok(),
        background: None,
        font_style: style,
    }
}

fn c(hex: &str) -> Option<Color> {
    Color::from_hex(hex).ok()
}

// ── Dark Modern token colors (from dark_plus / hc_black base) ────────────

fn dark_modern_tokens() -> Vec<TokenColorRule> {
    vec![
        tok("comment", "#6A9955"),
        tok("string", "#CE9178"),
        tok("string.regexp", "#D16969"),
        tok_multi(&["constant.numeric", "constant.other.color.rgb-value"], "#B5CEA8"),
        tok("constant.language", "#569CD6"),
        tok("constant.character", "#569CD6"),
        tok_multi(&["variable", "meta.definition.variable.name", "support.variable"], "#9CDCFE"),
        tok("meta.object-literal.key", "#9CDCFE"),
        tok("keyword", "#569CD6"),
        tok_multi(&["keyword.control", "keyword.other.using", "keyword.other.operator"], "#C586C0"),
        tok("keyword.operator", "#D4D4D4"),
        tok("storage", "#569CD6"),
        tok("storage.type", "#569CD6"),
        tok("storage.modifier", "#569CD6"),
        tok_multi(&["entity.name.function", "support.function"], "#DCDCAA"),
        tok_multi(&["entity.name.type", "entity.name.class", "support.class", "support.type"], "#4EC9B0"),
        tok_multi(&["meta.type.cast.expr", "entity.other.inherited-class"], "#4EC9B0"),
        tok("entity.name.tag", "#569CD6"),
        tok("entity.other.attribute-name", "#9CDCFE"),
        tok_multi(&["entity.other.attribute-name.class.css", "entity.other.attribute-name.id.css"], "#D7BA7D"),
        tok("support.constant", "#569CD6"),
        tok("punctuation.definition.tag", "#808080"),
        tok("meta.preprocessor", "#569CD6"),
        tok("meta.preprocessor.string", "#CE9178"),
        tok("meta.preprocessor.numeric", "#B5CEA8"),
        tok("variable.language.this", "#569CD6"),
        tok_styled("emphasis", "#D4D4D4", FontStyle::ITALIC),
        tok_styled("strong", "#D4D4D4", FontStyle::BOLD),
        tok_styled("markup.heading", "#6796E6", FontStyle::BOLD),
        tok("markup.inserted", "#B5CEA8"),
        tok("markup.deleted", "#CE9178"),
        tok("markup.changed", "#569CD6"),
        tok_styled("markup.italic", "#D4D4D4", FontStyle::ITALIC),
        tok_styled("markup.underline", "#D4D4D4", FontStyle::UNDERLINE),
    ]
}

fn light_modern_tokens() -> Vec<TokenColorRule> {
    vec![
        tok("comment", "#008000"),
        tok("string", "#A31515"),
        tok("string.regexp", "#811F3F"),
        tok("constant.numeric", "#098658"),
        tok("constant.language", "#0000FF"),
        tok("constant.character", "#0000FF"),
        tok_multi(&["variable", "meta.definition.variable.name"], "#001080"),
        tok("meta.object-literal.key", "#001080"),
        tok("keyword", "#0000FF"),
        tok_multi(&["keyword.control", "keyword.other.using"], "#AF00DB"),
        tok("keyword.operator", "#000000"),
        tok("storage", "#0000FF"),
        tok("storage.type", "#0000FF"),
        tok_multi(&["entity.name.function", "support.function"], "#795E26"),
        tok_multi(&["entity.name.type", "entity.name.class", "support.class", "support.type"], "#267F99"),
        tok_multi(&["meta.type.cast.expr", "entity.other.inherited-class"], "#267F99"),
        tok("entity.name.tag", "#800000"),
        tok("entity.other.attribute-name", "#E50000"),
        tok("support.constant", "#0000FF"),
        tok("punctuation.definition.tag", "#800000"),
        tok("meta.preprocessor", "#0000FF"),
        tok("meta.preprocessor.string", "#A31515"),
        tok("meta.preprocessor.numeric", "#098658"),
        tok("variable.language.this", "#0000FF"),
        tok_styled("emphasis", "#000000", FontStyle::ITALIC),
        tok_styled("strong", "#000000", FontStyle::BOLD),
        tok_styled("markup.heading", "#0451A5", FontStyle::BOLD),
        tok("markup.inserted", "#098658"),
        tok("markup.deleted", "#A31515"),
        tok("markup.changed", "#0451A5"),
    ]
}

fn hc_black_tokens() -> Vec<TokenColorRule> {
    vec![
        tok("comment", "#7CA668"),
        tok("string", "#CE9178"),
        tok("string.regexp", "#D16969"),
        tok_multi(&["constant.numeric", "constant.other.color.rgb-value"], "#B5CEA8"),
        tok("constant.language", "#569CD6"),
        tok("constant.character", "#569CD6"),
        tok("constant.regexp", "#B46695"),
        tok_multi(&["variable", "meta.definition.variable.name", "support.variable"], "#9CDCFE"),
        tok("keyword", "#569CD6"),
        tok_multi(&["keyword.control", "keyword.other.using", "keyword.other.operator"], "#C586C0"),
        tok("keyword.operator", "#D4D4D4"),
        tok("storage", "#569CD6"),
        tok("storage.type", "#569CD6"),
        tok("storage.modifier", "#569CD6"),
        tok_multi(&["entity.name.function", "support.function"], "#DCDCAA"),
        tok_multi(&["entity.name.type", "entity.name.class", "support.class", "support.type"], "#4EC9B0"),
        tok("entity.name.tag", "#569CD6"),
        tok_multi(&["entity.name.tag.css", "entity.name.tag.less"], "#D7BA7D"),
        tok("entity.other.attribute-name", "#9CDCFE"),
        tok("punctuation.definition.tag", "#808080"),
        tok("invalid", "#F44747"),
        tok_styled("emphasis", "#FFFFFF", FontStyle::ITALIC),
        tok_styled("strong", "#FFFFFF", FontStyle::BOLD),
        tok_styled("markup.heading", "#6796E6", FontStyle::BOLD),
        tok("markup.inserted", "#B5CEA8"),
        tok("markup.deleted", "#CE9178"),
        tok("markup.changed", "#569CD6"),
        tok_styled("markup.underline", "#FFFFFF", FontStyle::UNDERLINE),
        tok_styled("markup.strikethrough", "#FFFFFF", FontStyle::STRIKETHROUGH),
    ]
}

fn hc_light_tokens() -> Vec<TokenColorRule> {
    vec![
        tok("comment", "#515151"),
        tok_multi(&["string", "meta.embedded.assembly"], "#0F4A85"),
        tok("string.regexp", "#811F3F"),
        tok("constant.numeric", "#096D48"),
        tok("constant.language", "#0F4A85"),
        tok("constant.character", "#0F4A85"),
        tok_multi(&["variable", "meta.definition.variable.name", "support.variable"], "#001080"),
        tok("keyword", "#0F4A85"),
        tok_multi(&["keyword.control", "keyword.other.using"], "#B5200D"),
        tok("keyword.operator", "#000000"),
        tok("storage", "#0F4A85"),
        tok("storage.type", "#0F4A85"),
        tok_multi(&["entity.name.function", "support.function"], "#5E2CBC"),
        tok_multi(&["entity.name.type", "entity.name.class", "support.class", "support.type"], "#185E73"),
        tok("entity.name.tag", "#0F4A85"),
        tok("entity.other.attribute-name", "#264F78"),
        tok("punctuation.definition.tag", "#0F4A85"),
        tok("invalid", "#B5200D"),
        tok("variable.language", "#0F4A85"),
        tok("constant.character.escape", "#EE0000"),
        tok_styled("emphasis", "#000000", FontStyle::ITALIC),
        tok_styled("strong", "#000080", FontStyle::BOLD),
        tok_styled("markup.heading", "#0F4A85", FontStyle::BOLD),
        tok("markup.inserted", "#096D48"),
        tok("markup.deleted", "#5A5A5A"),
        tok("markup.changed", "#0451A5"),
        tok_styled("markup.italic", "#800080", FontStyle::ITALIC),
    ]
}

#[allow(clippy::too_many_lines)]
fn hc_black_colors() -> WorkbenchColors {
    WorkbenchColors {
        editor_background: c("#000000"),
        editor_foreground: c("#FFFFFF"),
        editor_selection_background: c("#FFFFFF"),
        editor_whitespace_foreground: c("#7c7c7c"),
        editor_indent_guide_background: c("#FFFFFF"),
        editor_indent_guide_active_background: c("#FFFFFF"),
        side_bar_title_foreground: c("#FFFFFF"),
        selection_background: c("#008000"),
        foreground: c("#FFFFFF"),
        focus_border: c("#F38518"),
        contrast_border: c("#6FC3DF"),
        contrast_active_border: c("#F38518"),
        error_foreground: c("#F48771"),
        text_link_foreground: c("#21A6FF"),
        text_link_active_foreground: c("#21A6FF"),
        icon_foreground: c("#FFFFFF"),
        ..WorkbenchColors::default()
    }
}

#[allow(clippy::too_many_lines)]
fn hc_light_colors() -> WorkbenchColors {
    WorkbenchColors {
        editor_background: c("#FFFFFF"),
        editor_foreground: c("#292929"),
        foreground: c("#292929"),
        focus_border: c("#006BBD"),
        contrast_border: c("#0F4A85"),
        contrast_active_border: c("#006BBD"),
        error_foreground: c("#B5200D"),
        text_link_foreground: c("#0F4A85"),
        text_link_active_foreground: c("#0F4A85"),
        icon_foreground: c("#292929"),
        status_bar_item_remote_background: c("#FFFFFF"),
        status_bar_item_remote_foreground: c("#000000"),
        ..WorkbenchColors::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_modern_loads() {
        let t = dark_modern();
        assert_eq!(t.kind, ThemeKind::Dark);
        assert!(!t.token_colors.is_empty());
        assert!(t.workbench_colors.editor_background.is_some());
    }

    #[test]
    fn light_modern_loads() {
        let t = light_modern();
        assert_eq!(t.kind, ThemeKind::Light);
        assert!(!t.token_colors.is_empty());
    }

    #[test]
    fn hc_black_loads() {
        let t = hc_black();
        assert_eq!(t.kind, ThemeKind::HighContrast);
        assert_eq!(t.workbench_colors.editor_background, c("#000000"));
    }

    #[test]
    fn hc_light_loads() {
        let t = hc_light();
        assert_eq!(t.kind, ThemeKind::HighContrastLight);
        assert_eq!(t.workbench_colors.editor_background, c("#FFFFFF"));
    }
}
