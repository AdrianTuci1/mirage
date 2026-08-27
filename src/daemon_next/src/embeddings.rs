use anyhow::{anyhow, Context, Result};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::{Tensor, Value};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Distribution, Normal};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const DEFAULT_EMBEDDING_DIM: usize = 384;
pub const DEFAULT_MAX_INPUT_LENGTH: usize = 128;

/// Strategy for producing dense vector embeddings from text.
pub trait Embedder: Send + Sync {
    /// Embed a single text fragment into a normalized dense vector.
    fn embed_text(&self, text: &str) -> Result<Vec<f32>>;

    /// Embed a batch of texts. Default implementation calls [`embed_text`] sequentially.
    fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed_text(t)).collect()
    }

    /// Expected vector dimensionality.
    fn dimension(&self) -> usize;
}

/// Factory that chooses between ONNX and fallback embeddings based on the contents of
/// `models_dir`. If no `.onnx` model is present, the fallback is used so tests and the
/// MVP pipeline can run without downloading models.
pub fn create_embedder(models_dir: impl AsRef<Path>) -> Result<Arc<dyn Embedder>> {
    let models_dir = models_dir.as_ref();
    if let Some(model_path) = find_onnx_model(models_dir) {
        match OnnxEmbedder::new(model_path) {
            Ok(onnx) => return Ok(Arc::new(onnx)),
            Err(err) => {
                tracing::warn!(
                    "Failed to load ONNX model at {}: {}. Using fallback embedder.",
                    models_dir.display(),
                    err
                );
            }
        }
    } else {
        tracing::info!(
            "No ONNX model found in {}. Using deterministic fallback embedder.",
            models_dir.display()
        );
    }
    Ok(Arc::new(FallbackEmbedder::new(DEFAULT_EMBEDDING_DIM)))
}

fn find_onnx_model(models_dir: &Path) -> Option<PathBuf> {
    if !models_dir.is_dir() {
        return None;
    }
    std::fs::read_dir(models_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("onnx"))
        .next()
}

/// ONNX Runtime backed embedder.
///
/// Loads the first `.onnx` model found in the supplied path. The model is expected to accept
/// transformer-style inputs (`input_ids`, `attention_mask`, optional `token_type_ids`) and
/// emit a `[batch, seq_len, dim]` or `[batch, dim]` float tensor.
pub struct OnnxEmbedder {
    session: Mutex<Session>,
    max_input_length: usize,
    embedding_dim: usize,
    input_names: Vec<String>,
}

impl OnnxEmbedder {
    pub fn new(model_path: impl AsRef<Path>) -> Result<Self> {
        let model_path = model_path.as_ref();
        let session = Session::builder()
            .map_err(|e| anyhow!("failed to create ONNX session builder: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow!("failed to set optimization level: {}", e))?
            .commit_from_file(model_path)
            .map_err(|e| anyhow!("failed to load ONNX model at {}: {}", model_path.display(), e))?;

        let input_names: Vec<String> = session.inputs().iter().map(|i| i.name().to_string()).collect();
        let max_input_length = DEFAULT_MAX_INPUT_LENGTH;
        let embedding_dim = DEFAULT_EMBEDDING_DIM;

        tracing::info!(
            "Loaded ONNX embedder from {} (inputs: {:?})",
            model_path.display(),
            input_names
        );
        Ok(Self {
            session: Mutex::new(session),
            max_input_length,
            embedding_dim,
            input_names,
        })
    }

    fn tokenize(&self, text: &str) -> Vec<i64> {
        text.split_whitespace()
            .filter(|s| !s.is_empty())
            .take(self.max_input_length)
            .map(|word| {
                let mut hash = 0_i64;
                for &b in word.as_bytes() {
                    hash = hash.wrapping_mul(31).wrapping_add(b as i64);
                }
                hash & 0x7FFFFFFF
            })
            .collect()
    }

    fn build_inputs(&self, text: &str) -> Result<HashMap<String, Value>> {
        let tokens = self.tokenize(text);
        let pad = self.max_input_length.saturating_sub(tokens.len());

        let input_ids: Vec<i64> = tokens
            .iter()
            .copied()
            .chain(std::iter::repeat(0_i64).take(pad))
            .collect();
        let attention_mask: Vec<i64> = std::iter::repeat(1_i64)
            .take(tokens.len())
            .chain(std::iter::repeat(0_i64).take(pad))
            .collect();
        let token_type_ids: Vec<i64> = std::iter::repeat(0_i64).take(self.max_input_length).collect();

        let input_ids_tensor =
            Tensor::from_array((vec![1_usize, self.max_input_length], input_ids))
                .context("failed to create input_ids tensor")?;
        let attention_mask_tensor =
            Tensor::from_array((vec![1_usize, self.max_input_length], attention_mask))
                .context("failed to create attention_mask tensor")?;
        let token_type_ids_tensor =
            Tensor::from_array((vec![1_usize, self.max_input_length], token_type_ids))
                .context("failed to create token_type_ids tensor")?;

        let mut inputs: HashMap<String, Value> = HashMap::new();
        if self.input_names.contains(&"input_ids".to_string()) {
            inputs.insert("input_ids".to_string(), input_ids_tensor.into());
        }
        if self.input_names.contains(&"attention_mask".to_string()) {
            inputs.insert("attention_mask".to_string(), attention_mask_tensor.into());
        }
        if self.input_names.contains(&"token_type_ids".to_string()) {
            inputs.insert("token_type_ids".to_string(), token_type_ids_tensor.into());
        }
        Ok(inputs)
    }

    fn extract_vector(&self, raw: &[f32]) -> Vec<f32> {
        let slice = if raw.len() >= self.embedding_dim {
            &raw[..self.embedding_dim]
        } else {
            raw
        };
        let mut v = slice.to_vec();
        v.resize(self.embedding_dim, 0.0_f32);
        normalize(&v)
    }
}

impl Embedder for OnnxEmbedder {
    fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(vec![0.0_f32; self.embedding_dim]);
        }

        let inputs = self.build_inputs(trimmed)?;
        let mut session = self
            .session
            .lock()
            .map_err(|e| anyhow!("failed to lock ONNX session: {}", e))?;
        let outputs = session.run(inputs).context("ONNX inference failed")?;

        let (_, value) = outputs
            .iter()
            .next()
            .context("ONNX model returned no outputs")?;
        let (_, data) = value
            .try_extract_tensor::<f32>()
            .context("failed to extract output tensor")?;

        // The output may be 2-D [batch, dim] or 3-D [batch, seq, dim].
        let flat: Vec<f32> = data.iter().copied().collect();
        Ok(self.extract_vector(&flat))
    }

    fn dimension(&self) -> usize {
        self.embedding_dim
    }
}

/// Deterministic fallback embedder used when no ONNX model is available.
pub struct FallbackEmbedder {
    dimension: usize,
}

impl FallbackEmbedder {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl Embedder for FallbackEmbedder {
    fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        if text.trim().is_empty() {
            return Ok(vec![0.0_f32; self.dimension]);
        }
        let seed = deterministic_seed(text);
        let mut rng = StdRng::seed_from_u64(seed);
        let normal = Normal::new(0.0_f32, 1.0_f32).expect("valid normal distribution");
        let mut values: Vec<f32> = (0..self.dimension)
            .map(|_| normal.sample(&mut rng))
            .collect();
        normalize_in_place(&mut values);
        Ok(values)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

fn deterministic_seed(text: &str) -> u64 {
    let digest = Sha256::digest(text.as_bytes());
    u64::from_be_bytes(digest[..8].try_into().expect("8 bytes from sha256"))
}

fn normalize(vector: &[f32]) -> Vec<f32> {
    let mut v = vector.to_vec();
    normalize_in_place(&mut v);
    v
}

fn normalize_in_place(vector: &mut [f32]) {
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in vector.iter_mut() {
            *x /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_is_deterministic() {
        let e = FallbackEmbedder::new(384);
        let a = e.embed_text("hello world").unwrap();
        let b = e.embed_text("hello world").unwrap();
        assert_eq!(a, b);
        assert!((a.iter().map(|x| x * x).sum::<f32>().sqrt() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn fallback_empty_text_is_zero() {
        let e = FallbackEmbedder::new(384);
        let v = e.embed_text("   ").unwrap();
        assert!(v.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn normalize_handles_zero() {
        let mut v = vec![0.0_f32; 10];
        normalize_in_place(&mut v);
        assert!(v.iter().all(|&x| x == 0.0));
    }
}
