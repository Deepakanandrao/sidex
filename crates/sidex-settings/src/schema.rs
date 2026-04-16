//! Settings schema definitions used for validation and UI generation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Describes the type of a setting value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SettingType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Enum { values: Vec<String> },
}

/// Schema describing a single setting: its key, type, default value, and
/// human-readable description.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingSchema {
    /// Dot-separated key, e.g. `"editor.fontSize"`.
    pub key: String,
    /// The value type.
    pub setting_type: SettingType,
    /// Default value as a JSON value.
    pub default: Value,
    /// Human-readable description shown in the settings UI.
    pub description: String,
}

/// Registry that collects setting schemas contributed by core modules and
/// extensions.
#[derive(Clone, Debug, Default)]
pub struct SchemaRegistry {
    schemas: Vec<SettingSchema>,
}

impl SchemaRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a setting schema.
    pub fn register(&mut self, schema: SettingSchema) {
        if let Some(existing) = self.schemas.iter_mut().find(|s| s.key == schema.key) {
            *existing = schema;
        } else {
            self.schemas.push(schema);
        }
    }

    /// Look up a schema by key.
    pub fn get(&self, key: &str) -> Option<&SettingSchema> {
        self.schemas.iter().find(|s| s.key == key)
    }

    /// Return all registered schemas.
    pub fn all(&self) -> &[SettingSchema] {
        &self.schemas
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_schema(key: &str) -> SettingSchema {
        SettingSchema {
            key: key.to_owned(),
            setting_type: SettingType::Number,
            default: json!(14),
            description: "Font size".to_owned(),
        }
    }

    #[test]
    fn register_and_get() {
        let mut reg = SchemaRegistry::new();
        reg.register(sample_schema("editor.fontSize"));
        assert!(reg.get("editor.fontSize").is_some());
        assert!(reg.get("editor.tabSize").is_none());
    }

    #[test]
    fn register_overwrites() {
        let mut reg = SchemaRegistry::new();
        reg.register(sample_schema("editor.fontSize"));
        let mut updated = sample_schema("editor.fontSize");
        updated.default = json!(16);
        reg.register(updated);
        assert_eq!(reg.get("editor.fontSize").unwrap().default, json!(16));
        assert_eq!(reg.all().len(), 1);
    }

    #[test]
    fn all_returns_everything() {
        let mut reg = SchemaRegistry::new();
        reg.register(sample_schema("a"));
        reg.register(sample_schema("b"));
        assert_eq!(reg.all().len(), 2);
    }
}
