use crate::analytics::Analytics;
use crate::db::LanceDbStore;
use crate::embeddings::Embedder;
use crate::slm::{AskResponse, SlmEngine};
use anyhow::{anyhow, Context, Result};
use serde_json::json;
use std::sync::Arc;

/// Deterministic SLM used when no ONNX model is downloaded.
///
/// It routes between semantic search and SQL using keyword heuristics and
/// produces a small natural-language summary for SQL results.
pub struct HeuristicSlmEngine {
    top_k: usize,
}

impl HeuristicSlmEngine {
    pub fn new(top_k: usize) -> Self {
        Self { top_k }
    }
}

#[async_trait::async_trait]
impl SlmEngine for HeuristicSlmEngine {
    async fn ask(
        &self,
        question: &str,
        store: Arc<LanceDbStore>,
        embedder: Arc<dyn Embedder>,
        analytics: Arc<Analytics>,
    ) -> Result<AskResponse> {
        let trimmed = question.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("question is empty"));
        }

        if is_sql_question(trimmed) {
            let (sql, summary) = heuristic_sql(trimmed, &analytics)?;
            let _ = sql;
            Ok(AskResponse::SqlQuery {
                question: trimmed.to_string(),
                natural_language_answer: summary,
            })
        } else {
            let vector = tokio::task::spawn_blocking({
                let embedder = Arc::clone(&embedder);
                let text = trimmed.to_string();
                move || embedder.embed_text(&text)
            })
            .await
            .context("embedding task panicked")?
            .context("failed to embed question")?;

            let raw = store
                .search(vector, self.top_k)
                .await
                .context("semantic search failed")?;
            let results: Vec<crate::models::SearchResult> = raw
                .into_iter()
                .map(|r| crate::models::SearchResult {
                    id: r.record.id,
                    relative_path: r.record.relative_path,
                    score: r.score,
                    source_type: r.record.source_type,
                    category: crate::models::SearchResultCategory::Semantic,
                    open_url: None,
                })
                .collect();

            Ok(AskResponse::SemanticSearch {
                question: trimmed.to_string(),
                results,
            })
        }
    }
}

fn is_sql_question(question: &str) -> bool {
    let lower = question.to_lowercase();
    let sql_keywords = [
        "how many", "count", "sum", "average", "avg", "total", "max", "min", "rows", "columns",
        "table", "csv", "parquet", "database", "sql", "select", "from", "where", "group", "order",
    ];
    sql_keywords.iter().any(|kw| lower.contains(kw))
}

fn heuristic_sql(question: &str, analytics: &Analytics) -> Result<(String, String)> {
    let lower = question.to_lowercase();

    // Discover available tables.
    let tables = analytics
        .query("SHOW TABLES")
        .context("failed to list tables")?;
    let table_names: Vec<&str> = tables
        .iter()
        .filter_map(|row| row.values().next().and_then(|v| v.as_str()))
        .collect();

    let chosen = table_names
        .iter()
        .find(|name| lower.contains(&name.to_lowercase()))
        .copied();

    let sql = if let Some(table) = chosen {
        if lower.contains("how many") || lower.contains("count") || lower.contains("rows") {
            format!("SELECT COUNT(*) AS total FROM {}", escape_identifier(table))
        } else {
            format!("SELECT * FROM {} LIMIT 10", escape_identifier(table))
        }
    } else {
        return Ok((
            String::new(),
            String::from("I couldn't match your question to a known table. Available tables: ")
                + &table_names.join(", "),
        ));
    };

    let rows = analytics
        .query(&sql)
        .context("heuristic SQL query failed")?;
    let summary = if rows.is_empty() {
        format!("No results found for '{}'.", question)
    } else {
        format!("Query returned {} row(s): {}", rows.len(), json!(rows))
    };

    Ok((sql, summary))
}

fn escape_identifier(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DaemonConfig;
    use tempfile::TempDir;

    fn temp_analytics() -> (Analytics, TempDir) {
        let dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            data_dir: dir.path().to_path_buf(),
            ..DaemonConfig::default()
        };
        (Analytics::open(&config).unwrap(), dir)
    }

    #[test]
    fn detects_sql_questions() {
        assert!(is_sql_question("how many photos do I have?"));
        assert!(is_sql_question("count rows in sales"));
        assert!(!is_sql_question("show me pictures of beaches"));
    }

    #[cfg(feature = "duckdb")]
    #[test]
    fn heuristic_sql_lists_tables() {
        let (analytics, _dir) = temp_analytics();
        analytics
            .execute("CREATE TABLE photos (id INTEGER)")
            .unwrap();
        analytics
            .execute("INSERT INTO photos VALUES (1), (2), (3)")
            .unwrap();

        let (_, summary) = heuristic_sql("how many photos?", &analytics).unwrap();
        assert!(summary.contains("3"));
    }
}
