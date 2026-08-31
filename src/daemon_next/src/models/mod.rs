use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub relative_path: String,
    pub source_type: String,
    pub vector: Vec<f32>,
    pub updated_at: String,
    pub version: i64,
    /// What produced the vector: `image`, `text` or `name`.
    #[serde(default = "default_modality")]
    pub modality: String,
    /// Short human readable summary of the content that was embedded.
    #[serde(default)]
    pub caption: String,
}

fn default_modality() -> String {
    crate::content::MediaKind::Metadata.as_str().to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchResultCategory {
    App,
    File,
    Semantic,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub relative_path: String,
    pub score: f64,
    pub source_type: String,
    #[serde(default = "default_search_category")]
    pub category: SearchResultCategory,
    #[serde(default)]
    pub open_url: Option<String>,
}

fn default_search_category() -> SearchResultCategory {
    SearchResultCategory::Semantic
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordWithScore {
    pub record: Record,
    pub score: f64,
}

/// A single row returned by the analytics SQL engine.
pub type QueryRow = serde_json::Map<String, Value>;
