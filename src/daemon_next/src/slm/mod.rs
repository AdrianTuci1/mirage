use crate::analytics::Analytics;
use crate::db::LanceDbStore;
use crate::embeddings::Embedder;
use crate::ipc::protocol::JsonRpcError;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod heuristic;
pub mod onnx;

pub use heuristic::HeuristicSlmEngine;
pub use onnx::OnnxSlmEngine;

/// Response produced by the SLM for a natural-language question.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AskResponse {
    /// The SLM decided the question is best answered by semantic search.
    #[serde(rename = "semantic_search")]
    SemanticSearch {
        question: String,
        results: Vec<crate::models::SearchResult>,
    },
    /// The SLM decided the question requires a SQL query over metadata.
    #[serde(rename = "sql_query")]
    SqlQuery {
        question: String,
        natural_language_answer: String,
    },
}

/// Trait for a natural-language router / SQL generator.
#[async_trait::async_trait]
pub trait SlmEngine: Send + Sync {
    /// Answer a user's natural-language question.
    async fn ask(
        &self,
        question: &str,
        store: Arc<LanceDbStore>,
        embedder: Arc<dyn Embedder>,
        analytics: Arc<Analytics>,
    ) -> Result<AskResponse>;
}

impl AskResponse {
    /// Convert an SLM error into a JSON-RPC error response.
    pub fn to_json_rpc_error(error: anyhow::Error) -> JsonRpcError {
        JsonRpcError::new(
            crate::ipc::protocol::ERROR_INTERNAL_ERROR,
            format!("ask failed: {}", error),
        )
    }
}
