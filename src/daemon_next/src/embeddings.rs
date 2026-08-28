use anyhow::{anyhow, Context, Result};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Distribution, Normal};
use sha2::{Digest, Sha256};
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

cfg_if::cfg_if! {
    if #[cfg(feature = "onnx")] {
        use ort::session::{builder::GraphOptimizationLevel, Session};
        use ort::value::{Tensor, Value};

        /// Factory that chooses between ONNX and fallback embeddings based on the contents of
        /// `models_dir`. If no `.onnx` model is present, the fallback is used so tests and the
        /// MVP pipeline can run without downloading models.
        pub fn create_embedder(models_dir: impl AsRef<Path>) -> Result<Arc<dyn Embedder>> {
            // If the onnx_runtime module was downloaded, point ort at the shared library.
            if let Some(runtime_path) = module_runtime_dylib("onnx_runtime") {
                let path_str = runtime_path.to_string_lossy().to_string();
                std::env::set_var("ORT_DYLIB_PATH", &path_str);
                tracing::info!("using downloaded ONNX Runtime library at {}", path_str);
            }

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

        /// Look for the downloaded ONNX Runtime shared library in the module install directory.
        fn module_runtime_dylib(module_id: &str) -> Option<PathBuf> {
            let base = std::env::var("MIRAGE_DOWNLOADS_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("downloads"));
            let dir = base.join(module_id);
            if !dir.is_dir() {
                return None;
            }
            let ext = if cfg!(target_os = "windows") { "dll" } else if cfg!(target_os = "macos") { "dylib" } else { "so" };
            let lib_name = if cfg!(target_os = "windows") {
                String::from("onnxruntime.dll")
            } else if cfg!(target_os = "macos") {
                String::from("libonnxruntime.dylib")
            } else {
                String::from("libonnxruntime.so")
            };
            let candidate = dir.join(&lib_name);
            if candidate.exists() { Some(candidate) } else { None }
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
        /// transformer-style inputs (`input_ids`, `attention_mask`, `token_type_ids`) and
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

            fn build_inputs(&self, text: &str) -> Result<Vec<(String, Value)>> {
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

                let mut inputs: Vec<(String, Value)> = Vec::new();
                if self.input_names.contains(&"input_ids".to_string()) {
                    inputs.push(("input_ids".to_string(), input_ids_tensor.into()));
                }
                if self.input_names.contains(&"attention_mask".to_string()) {
                    inputs.push(("attention_mask".to_string(), attention_mask_tensor.into()));
                }
                if self.input_names.contains(&"token_type_ids".to_string()) {
                    inputs.push(("token_type_ids".to_string(), token_type_ids_tensor.into()));
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
                normalize(&mut v)
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
    } else {
        /// Factory that always returns the deterministic fallback embedder when the `onnx`
        /// feature is disabled. The ONNX Runtime module can still be downloaded, but it cannot
        /// be used until the daemon is rebuilt with the `onnx` feature.
        pub fn create_embedder(_models_dir: impl AsRef<Path>) -> Result<Arc<dyn Embedder>> {
            Ok(Arc::new(FallbackEmbedder::new(DEFAULT_EMBEDDING_DIM)))
        }
    }
}

/// Deterministic fallback embedder that produces stable pseudo-random vectors.
///
/// Used when no ONNX model is available or when the `onnx` feature is disabled.
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
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(vec![0.0_f32; self.dimension]);
        }
        let mut hasher = Sha256::new();
        hasher.update(trimmed.as_bytes());
        let seed = <[u8; 32]>::try_from(&hasher.finalize()[..32]).unwrap();
        let rng = StdRng::from_seed(seed);
        let normal = Normal::new(0.0_f32, 1.0_f32).unwrap();
        let mut values: Vec<f32> = normal.sample_iter(rng).take(self.dimension).collect();
        Ok(normalize(&mut values))
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

fn normalize(v: &mut [f32]) -> Vec<f32> {
    let sum_sq: f32 = v.iter().map(|x| x * x).sum();
    if sum_sq == 0.0 {
        return v.to_vec();
    }
    let norm = sum_sq.sqrt();
    v.iter_mut().for_each(|x| *x /= norm);
    v.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_is_deterministic() {
        let embedder = FallbackEmbedder::new(8);
        let a = embedder.embed_text("hello").unwrap();
        let b = embedder.embed_text("hello").unwrap();
        assert_eq!(a, b);
    }
}
