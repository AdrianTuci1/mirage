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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub relative_path: String,
    pub score: f64,
    pub source_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordWithScore {
    pub record: Record,
    pub score: f64,
}

/// A single row returned by the analytics SQL engine.
pub type QueryRow = serde_json::Map<String, Value>;
