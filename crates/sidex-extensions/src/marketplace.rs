//! Marketplace API client for browsing and downloading extensions.
//!
//! Targets the Open VSX registry by default, with the base URL
//! configurable for alternative marketplaces.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "https://open-vsx.org/api";

/// Metadata about an extension as returned by the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceExtension {
    /// Canonical `namespace.name` id.
    #[serde(alias = "namespace_name")]
    pub id: String,
    /// Extension name.
    pub name: String,
    /// Latest version.
    pub version: String,
    /// Short description.
    #[serde(default)]
    pub description: String,
    /// Direct download URL for the `.vsix`.
    #[serde(default)]
    pub download_url: String,
    /// Icon URL.
    #[serde(default)]
    pub icon_url: String,
    /// Number of installs.
    #[serde(default)]
    pub install_count: u64,
    /// Average rating (0.0–5.0).
    #[serde(default)]
    pub rating: f64,
}

/// Client for querying an Open VSX-compatible marketplace.
pub struct MarketplaceClient {
    base_url: String,
    http: reqwest::Client,
}

impl MarketplaceClient {
    /// Creates a client pointing at the default Open VSX registry.
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_owned(),
            http: reqwest::Client::new(),
        }
    }

    /// Creates a client pointing at a custom marketplace URL.
    pub fn with_base_url(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            http: reqwest::Client::new(),
        }
    }

    /// Searches for extensions matching `query`.
    pub async fn search(
        &self,
        query: &str,
        page: u32,
    ) -> Result<Vec<MarketplaceExtension>> {
        let url = format!(
            "{base}/-/search?query={query}&offset={offset}&size=20",
            base = self.base_url,
            offset = page * 20,
        );

        let resp: SearchResponse = self
            .http
            .get(&url)
            .send()
            .await
            .context("marketplace search request failed")?
            .json()
            .await
            .context("failed to parse marketplace search response")?;

        Ok(resp.extensions)
    }

    /// Fetches metadata for a single extension by its id (`namespace.name`).
    pub async fn get_extension(&self, id: &str) -> Result<MarketplaceExtension> {
        let (namespace, name) = id
            .split_once('.')
            .unwrap_or(("unknown", id));

        let url = format!(
            "{base}/{namespace}/{name}",
            base = self.base_url,
        );

        let resp: MarketplaceExtension = self
            .http
            .get(&url)
            .send()
            .await
            .context("marketplace get_extension request failed")?
            .json()
            .await
            .context("failed to parse extension metadata")?;

        Ok(resp)
    }

    /// Downloads a `.vsix` for the given extension and version.
    pub async fn download_vsix(
        &self,
        id: &str,
        version: &str,
    ) -> Result<Vec<u8>> {
        let (namespace, name) = id
            .split_once('.')
            .unwrap_or(("unknown", id));

        let url = format!(
            "{base}/{namespace}/{name}/{version}/file/{namespace}.{name}-{version}.vsix",
            base = self.base_url,
        );

        let bytes = self
            .http
            .get(&url)
            .send()
            .await
            .context("vsix download request failed")?
            .bytes()
            .await
            .context("failed to read vsix bytes")?;

        Ok(bytes.to_vec())
    }
}

impl Default for MarketplaceClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal response shape for the Open VSX search endpoint.
#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    extensions: Vec<MarketplaceExtension>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_url() {
        let client = MarketplaceClient::new();
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn custom_base_url_strips_trailing_slash() {
        let client = MarketplaceClient::with_base_url("https://example.com/api/");
        assert_eq!(client.base_url, "https://example.com/api");
    }

    #[test]
    fn marketplace_extension_deserialize() {
        let json = r#"{
            "id": "rust-lang.rust-analyzer",
            "name": "rust-analyzer",
            "version": "0.4.1234",
            "description": "Rust language support",
            "downloadUrl": "https://example.com/file.vsix",
            "iconUrl": "https://example.com/icon.png",
            "installCount": 5000000,
            "rating": 4.8
        }"#;
        let ext: MarketplaceExtension = serde_json::from_str(json).unwrap();
        assert_eq!(ext.id, "rust-lang.rust-analyzer");
        assert_eq!(ext.name, "rust-analyzer");
        assert_eq!(ext.install_count, 5_000_000);
        assert!((ext.rating - 4.8).abs() < f64::EPSILON);
    }

    #[test]
    fn marketplace_extension_minimal() {
        let json = r#"{ "id": "a.b", "name": "b", "version": "1.0.0" }"#;
        let ext: MarketplaceExtension = serde_json::from_str(json).unwrap();
        assert_eq!(ext.id, "a.b");
        assert!(ext.description.is_empty());
        assert_eq!(ext.install_count, 0);
    }
}
