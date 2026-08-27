use crate::analytics::Analytics;
use crate::db::LanceDbStore;
use crate::embeddings::Embedder;
use crate::slm::{AskResponse, SlmEngine};
use anyhow::{anyhow, Context, Result};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::{Shape, Tensor, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;

/// Directory layout expected for the `slm_nl_router` module.
const TOKENIZER_FILE: &str = "tokenizer.json";
const ENCODER_FILE: &str = "encoder.onnx";
const DECODER_FILE: &str = "decoder.onnx";
const SINGLE_MODEL_FILE: &str = "model.onnx";

/// ONNX-backed SLM using a T5/mt0-small architecture.
///
/// Expects files:
///   `<models_dir>/slm_nl_router/<version>/tokenizer.json`
///   `<models_dir>/slm_nl_router/<version>/encoder.onnx`
///   `<models_dir>/slm_nl_router/<version>/decoder.onnx`
///
/// If only `model.onnx` is present, the engine reports that it needs a
/// split encoder/decoder export.
pub struct OnnxSlmEngine {
    inference: T5Inference,
    top_k: usize,
}

impl OnnxSlmEngine {
    /// Try to load the SLM from the standard module layout under `models_dir`.
    /// Returns an error if files are missing or invalid.
    pub fn new(models_dir: impl AsRef<Path>) -> Result<Self> {
        let models_dir = models_dir.as_ref();
        let module_dir = find_module_dir(models_dir)?;
        let tokenizer_path = module_dir.join(TOKENIZER_FILE);
        let encoder_path = module_dir.join(ENCODER_FILE);
        let decoder_path = module_dir.join(DECODER_FILE);

        if !tokenizer_path.exists() {
            return Err(anyhow!(
                "tokenizer not found at {}",
                tokenizer_path.display()
            ));
        }
        if !encoder_path.exists() || !decoder_path.exists() {
            let single = module_dir.join(SINGLE_MODEL_FILE);
            if single.exists() {
                return Err(anyhow!(
                    "found single model.onnx but OnnxSlmEngine requires separate encoder.onnx and decoder.onnx"
                ));
            }
            return Err(anyhow!(
                "encoder/decoder ONNX files not found in {}",
                module_dir.display()
            ));
        }

        let inference = T5Inference::new(&tokenizer_path, &encoder_path, &decoder_path)?;
        Ok(Self { inference, top_k: 10 })
    }

    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k;
        self
    }
}

#[async_trait::async_trait]
impl SlmEngine for OnnxSlmEngine {
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

        let tables = list_tables(&analytics)?;
        let prompt = build_prompt(trimmed, &tables);
        let generated = self.inference.generate(&prompt)?;

        parse_model_output(&generated, trimmed, store, embedder, analytics, self.top_k).await
    }
}

/// T5-style encoder/decoder inference without KV cache.
struct T5Inference {
    tokenizer: Tokenizer,
    encoder: Mutex<Session>,
    decoder: Mutex<Session>,
    pad_token_id: i64,
    eos_token_id: i64,
    max_length: usize,
}

impl T5Inference {
    fn new(
        tokenizer_path: impl AsRef<Path>,
        encoder_path: impl AsRef<Path>,
        decoder_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow!("failed to load tokenizer: {}", e))?;

        let encoder = Session::builder()
            .map_err(|e| anyhow!("failed to create encoder session builder: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow!("failed to set encoder optimization: {}", e))?
            .commit_from_file(encoder_path)
            .map_err(|e| anyhow!("failed to load encoder ONNX: {}", e))?;

        let decoder = Session::builder()
            .map_err(|e| anyhow!("failed to create decoder session builder: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow!("failed to set decoder optimization: {}", e))?
            .commit_from_file(decoder_path)
            .map_err(|e| anyhow!("failed to load decoder ONNX: {}", e))?;

        // T5 / mt0 defaults: pad=0, eos=1.
        let pad_token_id = tokenizer
            .token_to_id("<pad>")
            .map(|u| u as i64)
            .unwrap_or(0_i64);
        let eos_token_id = tokenizer
            .token_to_id("</s>")
            .map(|u| u as i64)
            .unwrap_or(1_i64);

        Ok(Self {
            tokenizer,
            encoder: Mutex::new(encoder),
            decoder: Mutex::new(decoder),
            pad_token_id,
            eos_token_id,
            max_length: 128,
        })
    }

    fn generate(&self, prompt: &str) -> Result<String> {
        let encoding = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| anyhow!("failed to encode prompt: {}", e))?;
        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&u| u as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&u| u as i64)
            .collect();

        let (encoder_shape, encoder_hidden_states) = self.run_encoder(&input_ids, &attention_mask)?;

        let mut decoder_ids = vec![self.pad_token_id];
        let mut decoder_mask = vec![1_i64];

        for _ in 0..self.max_length {
            let (logits_shape, logits) = self.run_decoder(
                &decoder_ids,
                &decoder_mask,
                &attention_mask,
                &encoder_shape,
                &encoder_hidden_states,
            )?;

            let next_token = argmax_last(&logits, &logits_shape);
            if next_token == self.eos_token_id {
                break;
            }
            decoder_ids.push(next_token);
            decoder_mask.push(1_i64);
        }

        // Skip the initial pad/start token for decoding.
        let output_ids: Vec<u32> = decoder_ids
            .iter()
            .skip(1)
            .map(|&i| i as u32)
            .collect();

        self.tokenizer
            .decode(&output_ids, true)
            .map_err(|e| anyhow!("failed to decode output: {}", e))
    }

    fn run_encoder(
        &self,
        input_ids: &[i64],
        attention_mask: &[i64],
    ) -> Result<(Shape, Vec<f32>)> {
        let seq_len = input_ids.len();
        let input_ids_tensor =
            Tensor::from_array((vec![1_usize, seq_len], input_ids.to_vec()))
                .context("failed to create encoder input_ids tensor")?;
        let attention_mask_tensor =
            Tensor::from_array((vec![1_usize, seq_len], attention_mask.to_vec()))
                .context("failed to create encoder attention_mask tensor")?;

        let mut encoder = self
            .encoder
            .lock()
            .map_err(|e| anyhow!("failed to lock encoder session: {}", e))?;

        let mut inputs: HashMap<String, Value> = HashMap::new();
        for input in encoder.inputs() {
            let name = input.name();
            if name == "input_ids" {
                inputs.insert(name.to_string(), input_ids_tensor.clone().into());
            } else if name == "attention_mask" {
                inputs.insert(name.to_string(), attention_mask_tensor.clone().into());
            }
        }

        let outputs = encoder
            .run(inputs)
            .map_err(|e| anyhow!("encoder inference failed: {}", e))?;

        let (_, value) = outputs.iter().next().context("encoder returned no outputs")?;
        let (shape, data) = value
            .try_extract_tensor::<f32>()
            .context("failed to extract encoder hidden states")?;
        Ok((shape.clone(), data.iter().copied().collect()))
    }

    fn run_decoder(
        &self,
        decoder_ids: &[i64],
        decoder_mask: &[i64],
        encoder_mask: &[i64],
        encoder_shape: &Shape,
        encoder_hidden_states: &[f32],
    ) -> Result<(Shape, Vec<f32>)> {
        let decoder_seq_len = decoder_ids.len();
        let encoder_seq_len = encoder_mask.len();
        let hidden_shape: Vec<usize> = encoder_shape.iter().map(|&d| d as usize).collect();

        let decoder_ids_tensor =
            Tensor::from_array((vec![1_usize, decoder_seq_len], decoder_ids.to_vec()))
                .context("failed to create decoder input_ids tensor")?;
        let decoder_mask_tensor =
            Tensor::from_array((vec![1_usize, decoder_seq_len], decoder_mask.to_vec()))
                .context("failed to create decoder attention_mask tensor")?;
        let encoder_mask_tensor =
            Tensor::from_array((vec![1_usize, encoder_seq_len], encoder_mask.to_vec()))
                .context("failed to create encoder attention_mask tensor")?;
        let hidden_tensor =
            Tensor::from_array((hidden_shape.clone(), encoder_hidden_states.to_vec()))
                .context("failed to create encoder_hidden_states tensor")?;

        let mut decoder = self
            .decoder
            .lock()
            .map_err(|e| anyhow!("failed to lock decoder session: {}", e))?;

        let mut inputs: HashMap<String, Value> = HashMap::new();
        for input in decoder.inputs() {
            let name = input.name();
            if name == "input_ids" || name == "decoder_input_ids" {
                inputs.insert(name.to_string(), decoder_ids_tensor.clone().into());
            } else if name == "attention_mask" || name == "decoder_attention_mask" {
                inputs.insert(name.to_string(), decoder_mask_tensor.clone().into());
            } else if name == "encoder_attention_mask" || name == "encoder_mask" {
                inputs.insert(name.to_string(), encoder_mask_tensor.clone().into());
            } else if name == "encoder_hidden_states"
                || name == "encoder_outputs"
                || name == "hidden_states"
            {
                inputs.insert(name.to_string(), hidden_tensor.clone().into());
            }
        }

        let outputs = decoder
            .run(inputs)
            .map_err(|e| anyhow!("decoder inference failed: {}", e))?;

        let (_, value) = outputs.iter().next().context("decoder returned no outputs")?;
        let (shape, data) = value
            .try_extract_tensor::<f32>()
            .context("failed to extract decoder logits")?;
        Ok((shape.clone(), data.iter().copied().collect()))
    }
}

fn find_module_dir(models_dir: &Path) -> Result<PathBuf> {
    let base = models_dir.join("slm_nl_router");
    if !base.exists() {
        return Err(anyhow!(
            "slm_nl_router module directory not found at {}",
            base.display()
        ));
    }
    // Pick the first version directory found.
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&base)
        .with_context(|| format!("failed to read {}", base.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();
    entries
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no version directory under {}", base.display()))
}

fn list_tables(analytics: &Analytics) -> Result<Vec<String>> {
    let rows = analytics.query("SHOW TABLES").context("failed to list tables")?;
    Ok(rows
        .into_iter()
        .filter_map(|row| row.values().next().cloned())
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect())
}

fn build_prompt(question: &str, tables: &[String]) -> String {
    let tables_str = if tables.is_empty() {
        String::from("none")
    } else {
        tables.join(", ")
    };
    format!(
        "Question: {}\nTables: {}\n\n\
         Decide: semantic_search or sql_query.\n\
         If semantic_search, respond: ACTION: semantic_search\n\
         If sql_query, respond with one SQL line prefixed SQL: and one answer line prefixed ANSWER:",
        question, tables_str
    )
}

async fn parse_model_output(
    text: &str,
    question: &str,
    store: Arc<LanceDbStore>,
    embedder: Arc<dyn Embedder>,
    analytics: Arc<Analytics>,
    top_k: usize,
) -> Result<AskResponse> {
    let lower = text.to_lowercase();

    if lower.contains("semantic_search") || lower.contains("semantic search") {
        return run_semantic_search(question, store, embedder, top_k).await;
    }

    if let Some(sql) = extract_sql(text) {
        let rows = analytics
            .query(&sql)
            .with_context(|| format!("SLM SQL query failed: {}", sql))?;
        let answer = extract_answer(text).unwrap_or_else(|| {
            if rows.is_empty() {
                format!("No results found for '{}'.", question)
            } else {
                format!("Query returned {} row(s).", rows.len())
            }
        });
        return Ok(AskResponse::SqlQuery {
            question: question.to_string(),
            natural_language_answer: answer,
        });
    }

    // Fallback to semantic search if the model output is unparseable.
    run_semantic_search(question, store, embedder, top_k).await
}

async fn run_semantic_search(
    question: &str,
    store: Arc<LanceDbStore>,
    embedder: Arc<dyn Embedder>,
    top_k: usize,
) -> Result<AskResponse> {
    let vector = tokio::task::spawn_blocking({
        let embedder = Arc::clone(&embedder);
        let text = question.to_string();
        move || embedder.embed_text(&text)
    })
    .await
    .context("embedding task panicked")?
    .context("failed to embed question")?;

    let raw = store
        .search(vector, top_k)
        .await
        .context("semantic search failed")?;
    let results: Vec<crate::models::SearchResult> = raw
        .into_iter()
        .map(|r| crate::models::SearchResult {
            id: r.record.id,
            relative_path: r.record.relative_path,
            score: r.score,
            source_type: r.record.source_type,
        })
        .collect();

    Ok(AskResponse::SemanticSearch {
        question: question.to_string(),
        results,
    })
}

fn extract_sql(text: &str) -> Option<String> {
    for line in text.lines() {
        if line.to_lowercase().starts_with("sql:") {
            return Some(line[4..].trim().to_string());
        }
    }
    None
}

fn extract_answer(text: &str) -> Option<String> {
    for line in text.lines() {
        if line.to_lowercase().starts_with("answer:") {
            return Some(line[7..].trim().to_string());
        }
    }
    None
}

fn argmax_last(logits: &[f32], shape: &Shape) -> i64 {
    // logits shape is typically [batch=1, seq_len, vocab_size].
    // Take the logits for the last generated position.
    let dims: Vec<usize> = shape.iter().map(|&d| d as usize).collect();
    let vocab_size = dims.last().copied().unwrap_or(logits.len()).max(1);
    let start = logits.len().saturating_sub(vocab_size);
    let last_slice = &logits[start..];
    last_slice
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(idx, _)| idx as i64)
        .unwrap_or(0_i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_sql_and_answer() {
        let text = "SQL: SELECT COUNT(*) FROM photos\nANSWER: You have 42 photos.";
        assert_eq!(
            extract_sql(text),
            Some(String::from("SELECT COUNT(*) FROM photos"))
        );
        assert_eq!(
            extract_answer(text),
            Some(String::from("You have 42 photos."))
        );
    }

    #[test]
    fn build_prompt_includes_tables() {
        let prompt = build_prompt("how many photos?", &[String::from("photos"), String::from("videos")]);
        assert!(prompt.contains("how many photos?"));
        assert!(prompt.contains("photos, videos"));
        assert!(prompt.contains("ACTION: semantic_search"));
    }

    #[test]
    fn argmax_last_picks_last_position_argmax() {
        // Shape [1, 3, 5] -> data length 15, vocab_size 5, last position at index 10..15.
        let logits: Vec<f32> = (0..15).map(|i| i as f32).collect();
        let shape = Shape::from(vec![1_i64, 3_i64, 5_i64]);
        let token = argmax_last(&logits, &shape);
        assert_eq!(token, 4); // max of [10,11,12,13,14]
    }

    #[test]
    fn argmax_last_handles_two_dimensional_logits() {
        // Shape [2, 4] -> data length 8, vocab_size 4, last position at index 4..8.
        let logits = vec![0.0_f32, 0.1, 0.2, 0.3, 1.0, 2.0, 0.5, 0.4];
        let shape = Shape::from(vec![2_i64, 4_i64]);
        let token = argmax_last(&logits, &shape);
        assert_eq!(token, 1); // max of [1.0, 2.0, 0.5, 0.4]
    }
}
