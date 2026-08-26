use serde::{Deserialize, Serialize};

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
