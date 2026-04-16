//! Navigation features wrapping LSP go-to and find-references requests.
//!
//! Provides a unified API for definition, declaration, implementation,
//! type definition, and reference lookup.

use std::str::FromStr;

use anyhow::{Context, Result};
use lsp_types::{
    GotoDefinitionResponse, PartialResultParams, ReferenceContext, ReferenceParams,
    TextDocumentIdentifier, TextDocumentPositionParams, Uri, WorkDoneProgressParams,
};
use serde::{Deserialize, Serialize};

use crate::client::LspClient;
use crate::conversion::{lsp_to_range, position_to_lsp};

/// A source location (file URI + range).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// File URI (e.g. `"file:///home/user/project/src/main.rs"`).
    pub uri: String,
    /// Range within the file.
    pub range: sidex_text::Range,
}

/// `textDocument/definition`
pub async fn goto_definition(
    client: &LspClient,
    uri: &str,
    pos: sidex_text::Position,
) -> Result<Vec<Location>> {
    let lsp_pos = position_to_lsp(pos);
    let response = client.goto_definition(uri, lsp_pos).await?;
    Ok(convert_goto_response(response))
}

/// `textDocument/declaration`
pub async fn goto_declaration(
    client: &LspClient,
    uri: &str,
    pos: sidex_text::Position,
) -> Result<Vec<Location>> {
    let lsp_pos = position_to_lsp(pos);
    let params = lsp_types::GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier::new(Uri::from_str(uri).context("invalid URI")?),
            position: lsp_pos,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let val = serde_json::to_value(params)?;
    let result = client
        .raw_request("textDocument/declaration", Some(val))
        .await?;
    if result.is_null() {
        return Ok(vec![]);
    }
    let response: GotoDefinitionResponse =
        serde_json::from_value(result).context("failed to parse declaration response")?;
    Ok(convert_goto_response(response))
}

/// `textDocument/implementation`
pub async fn goto_implementation(
    client: &LspClient,
    uri: &str,
    pos: sidex_text::Position,
) -> Result<Vec<Location>> {
    let lsp_pos = position_to_lsp(pos);
    let params = lsp_types::GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier::new(Uri::from_str(uri).context("invalid URI")?),
            position: lsp_pos,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let val = serde_json::to_value(params)?;
    let result = client
        .raw_request("textDocument/implementation", Some(val))
        .await?;
    if result.is_null() {
        return Ok(vec![]);
    }
    let response: GotoDefinitionResponse =
        serde_json::from_value(result).context("failed to parse implementation response")?;
    Ok(convert_goto_response(response))
}

/// `textDocument/typeDefinition`
pub async fn goto_type_definition(
    client: &LspClient,
    uri: &str,
    pos: sidex_text::Position,
) -> Result<Vec<Location>> {
    let lsp_pos = position_to_lsp(pos);
    let params = lsp_types::GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier::new(Uri::from_str(uri).context("invalid URI")?),
            position: lsp_pos,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let val = serde_json::to_value(params)?;
    let result = client
        .raw_request("textDocument/typeDefinition", Some(val))
        .await?;
    if result.is_null() {
        return Ok(vec![]);
    }
    let response: GotoDefinitionResponse =
        serde_json::from_value(result).context("failed to parse typeDefinition response")?;
    Ok(convert_goto_response(response))
}

/// `textDocument/references`
pub async fn find_references(
    client: &LspClient,
    uri: &str,
    pos: sidex_text::Position,
    include_declaration: bool,
) -> Result<Vec<Location>> {
    let lsp_pos = position_to_lsp(pos);
    let params = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier::new(Uri::from_str(uri).context("invalid URI")?),
            position: lsp_pos,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext {
            include_declaration,
        },
    };
    let val = serde_json::to_value(params)?;
    let result = client
        .raw_request("textDocument/references", Some(val))
        .await?;
    if result.is_null() {
        return Ok(vec![]);
    }
    let locations: Vec<lsp_types::Location> =
        serde_json::from_value(result).context("failed to parse references response")?;
    Ok(locations.iter().map(convert_location).collect())
}

fn convert_goto_response(response: GotoDefinitionResponse) -> Vec<Location> {
    match response {
        GotoDefinitionResponse::Scalar(loc) => vec![convert_location(&loc)],
        GotoDefinitionResponse::Array(locs) => locs.iter().map(convert_location).collect(),
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|link| Location {
                uri: link.target_uri.to_string(),
                range: lsp_to_range(link.target_selection_range),
            })
            .collect(),
    }
}

fn convert_location(loc: &lsp_types::Location) -> Location {
    Location {
        uri: loc.uri.to_string(),
        range: lsp_to_range(loc.range),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_scalar_response() {
        let response = GotoDefinitionResponse::Scalar(lsp_types::Location {
            uri: "file:///test.rs".parse().unwrap(),
            range: lsp_types::Range::new(
                lsp_types::Position::new(5, 0),
                lsp_types::Position::new(5, 10),
            ),
        });
        let locations = convert_goto_response(response);
        assert_eq!(locations.len(), 1);
        assert!(locations[0].uri.contains("test.rs"));
        assert_eq!(locations[0].range.start.line, 5);
    }

    #[test]
    fn convert_array_response() {
        let response = GotoDefinitionResponse::Array(vec![
            lsp_types::Location {
                uri: "file:///a.rs".parse().unwrap(),
                range: lsp_types::Range::new(
                    lsp_types::Position::new(1, 0),
                    lsp_types::Position::new(1, 5),
                ),
            },
            lsp_types::Location {
                uri: "file:///b.rs".parse().unwrap(),
                range: lsp_types::Range::new(
                    lsp_types::Position::new(10, 0),
                    lsp_types::Position::new(10, 5),
                ),
            },
        ]);
        let locations = convert_goto_response(response);
        assert_eq!(locations.len(), 2);
    }

    #[test]
    fn convert_link_response() {
        let response = GotoDefinitionResponse::Link(vec![lsp_types::LocationLink {
            origin_selection_range: None,
            target_uri: "file:///target.rs".parse().unwrap(),
            target_range: lsp_types::Range::new(
                lsp_types::Position::new(0, 0),
                lsp_types::Position::new(10, 0),
            ),
            target_selection_range: lsp_types::Range::new(
                lsp_types::Position::new(3, 4),
                lsp_types::Position::new(3, 15),
            ),
        }]);
        let locations = convert_goto_response(response);
        assert_eq!(locations.len(), 1);
        assert!(locations[0].uri.contains("target.rs"));
        assert_eq!(locations[0].range.start.line, 3);
        assert_eq!(locations[0].range.start.column, 4);
    }

    #[test]
    fn location_serialize() {
        let loc = Location {
            uri: "file:///test.rs".into(),
            range: sidex_text::Range::new(
                sidex_text::Position::new(1, 2),
                sidex_text::Position::new(3, 4),
            ),
        };
        let json = serde_json::to_string(&loc).unwrap();
        let back: Location = serde_json::from_str(&json).unwrap();
        assert_eq!(loc, back);
    }

    #[test]
    fn empty_array_response() {
        let response = GotoDefinitionResponse::Array(vec![]);
        let locations = convert_goto_response(response);
        assert!(locations.is_empty());
    }
}
