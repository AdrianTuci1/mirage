use anyhow::{anyhow, Context, Result};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Distribution, Normal};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const DEFAULT_EMBEDDING_DIM: usize = 384;
pub const DEFAULT_MAX_INPUT_LENGTH: usize = 128;

/// CLIP context length: 77 tokens including the start and end markers.
pub const CLIP_CONTEXT_LENGTH: usize = 77;
/// Vector size produced by `clip-vit-base-patch32` on both sides.
pub const CLIP_EMBEDDING_DIM: usize = 512;
/// Square crop fed to the vision encoder.
pub const CLIP_IMAGE_SIZE: usize = 224;
/// CLIP pads with token 0 (`!`), never with the end-of-text marker.
pub const CLIP_PAD_TOKEN: i64 = 0;
const CLIP_MEAN: [f32; 3] = [0.48145466, 0.4578275, 0.40821073];
const CLIP_STD: [f32; 3] = [0.26862954, 0.26130258, 0.27576711];
/// Caps on one inference batch. Attention activations dominate memory and grow with
/// the batch, so the budget-derived size is clamped to these.
const CLIP_TEXT_BATCH_CAP: usize = 32;
const CLIP_IMAGE_BATCH_CAP: usize = 8;
/// Rough transient cost of one image inside the vision encoder, in megabytes.
const CLIP_IMAGE_MB: usize = 96;
/// Model overhead reserved from the memory budget before batching.
const MODEL_OVERHEAD_MB: usize = 1536;

/// Strategy for producing dense vectors in a shared text-image space.
///
/// All vectors returned by one implementation live in the same space, so a text
/// query can be compared directly against an image vector.
pub trait Embedder: Send + Sync {
    /// Embed a single text fragment into a normalized dense vector.
    fn embed_text(&self, text: &str) -> Result<Vec<f32>>;

    /// Embed a batch of texts. Default implementation calls [`embed_text`] sequentially.
    fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed_text(t)).collect()
    }

    /// Embed a large list of texts in memory-bounded sub-batches.
    ///
    /// The default implementation derives a sub-batch size from the supplied memory
    /// budget, the dimensionality and the maximum input length, so the transient
    /// allocation for one batch stays well below the budget.
    fn embed_texts_batched(
        &self,
        texts: &[String],
        memory_budget_mb: usize,
    ) -> Result<Vec<Vec<f32>>> {
        let sub_batch = estimate_embedding_batch_size(
            memory_budget_mb,
            self.dimension(),
            DEFAULT_MAX_INPUT_LENGTH,
        );
        let mut results = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(sub_batch.max(1)) {
            results.extend(self.embed_texts(chunk)?);
        }
        Ok(results)
    }

    /// Embed the contents of an image file.
    fn embed_image_file(&self, _path: &Path) -> Result<Vec<f32>> {
        Err(anyhow!(
            "the active embedder cannot process images ({} dimensions of text-only space)",
            self.dimension()
        ))
    }

    /// Embed several images. Errors are reported per file so one unreadable image
    /// does not abort a whole indexing batch.
    fn embed_image_files(&self, paths: &[PathBuf]) -> Vec<Result<Vec<f32>>> {
        paths.iter().map(|p| self.embed_image_file(p)).collect()
    }

    /// Whether [`Embedder::embed_image_file`] is implemented for this embedder.
    fn supports_images(&self) -> bool {
        false
    }

    /// Whether vectors come from a real model rather than a deterministic stand-in.
    ///
    /// The fallback embedder matches strings, not meaning, and callers must not
    /// present its results as semantic search.
    fn is_semantic(&self) -> bool {
        true
    }

    /// Expected vector dimensionality.
    fn dimension(&self) -> usize;
}

/// Estimate how many texts can be embedded in one batch without exceeding the
/// supplied memory budget. Reserves room for model weights / runtime overhead
/// and assumes worst-case transient tensors for the ONNX pipeline.
pub fn estimate_embedding_batch_size(
    memory_budget_mb: usize,
    dimension: usize,
    max_input_length: usize,
) -> usize {
    let usable_mb = memory_budget_mb.saturating_sub(MODEL_OVERHEAD_MB);
    if usable_mb == 0 {
        return 1;
    }
    let usable_bytes = usable_mb * 1024 * 1024;
    // Inputs: input_ids, attention_mask, token_type_ids (i64 each).
    // Output vector (f32). Multiply by 2 to account for framework temporaries.
    let per_item_bytes = (3 * max_input_length * 8 + dimension * 4) * 2;
    if per_item_bytes == 0 {
        return 1024;
    }
    (usable_bytes / per_item_bytes as usize).max(1)
}

/// Number of images to decode and feed through the vision encoder at once.
pub fn estimate_image_batch_size(memory_budget_mb: usize) -> usize {
    let usable_mb = memory_budget_mb.saturating_sub(MODEL_OVERHEAD_MB);
    (usable_mb / CLIP_IMAGE_MB).clamp(1, CLIP_IMAGE_BATCH_CAP)
}

/// Pad or truncate CLIP token ids to the model context length.
///
/// Long inputs keep the leading tokens and the trailing end-of-text marker; the
/// remainder is padded with [`CLIP_PAD_TOKEN`] and masked out.
pub fn clip_align_tokens(ids: &[i64]) -> (Vec<i64>, Vec<i64>) {
    let mut aligned: Vec<i64> = Vec::with_capacity(CLIP_CONTEXT_LENGTH);
    let context = CLIP_CONTEXT_LENGTH;
    if ids.len() > context {
        aligned.extend_from_slice(&ids[..context - 1]);
        if let Some(&last) = ids.last() {
            aligned.push(last);
        }
    } else {
        aligned.extend_from_slice(ids);
    }
    let real = aligned.len();
    let mask = {
        let mut m = vec![1_i64; real];
        m.resize(context, 0);
        m
    };
    aligned.resize(context, CLIP_PAD_TOKEN);
    (aligned, mask)
}

/// Turn an RGB image into the normalized CHW float buffer the vision encoder expects.
///
/// The shortest side is scaled to [`CLIP_IMAGE_SIZE`] and the result is center-cropped,
/// matching the reference CLIP preprocessing.
pub fn clip_pixel_values(image: &image::RgbImage) -> Vec<f32> {
    let width = image.width().max(1) as f32;
    let height = image.height().max(1) as f32;
    let scale = CLIP_IMAGE_SIZE as f32 / width.min(height);
    let resized_w = ((width * scale).round() as u32).max(CLIP_IMAGE_SIZE as u32);
    let resized_h = ((height * scale).round() as u32).max(CLIP_IMAGE_SIZE as u32);
    let resized = image::imageops::resize(
        image,
        resized_w,
        resized_h,
        image::imageops::FilterType::Triangle,
    );
    let left = (resized_w - CLIP_IMAGE_SIZE as u32) / 2;
    let top = (resized_h - CLIP_IMAGE_SIZE as u32) / 2;
    let cropped = image::imageops::crop_imm(
        &resized,
        left,
        top,
        CLIP_IMAGE_SIZE as u32,
        CLIP_IMAGE_SIZE as u32,
    )
    .to_image();

    let plane = CLIP_IMAGE_SIZE * CLIP_IMAGE_SIZE;
    let mut data = vec![0_f32; 3 * plane];
    for (index, pixel) in cropped.as_raw().chunks(3).enumerate() {
        for channel in 0..3 {
            let value = pixel[channel] as f32 / 255.0;
            data[channel * plane + index] = (value - CLIP_MEAN[channel]) / CLIP_STD[channel];
        }
    }
    data
}

/// Decode an image file and prepare its encoder input.
pub fn load_clip_pixel_values(path: &Path) -> Result<Vec<f32>> {
    let reader = image::ImageReader::open(path)
        .map_err(|e| anyhow!("failed to open image {}: {}", path.display(), e))?
        .with_guessed_format()
        .map_err(|e| anyhow!("failed to guess image format for {}: {}", path.display(), e))?;
    let decoded = reader
        .decode()
        .map_err(|e| anyhow!("failed to decode image {}: {}", path.display(), e))?;
    let rgb = decoded.to_rgb8();
    if rgb.width() == 0 || rgb.height() == 0 {
        return Err(anyhow!("image {} has an empty frame", path.display()));
    }
    Ok(clip_pixel_values(&rgb))
}

cfg_if::cfg_if! {
    if #[cfg(feature = "onnx")] {
        use ort::session::{builder::GraphOptimizationLevel, Session};
        use ort::value::{Tensor, Value};
        use tokenizers::Tokenizer;

        /// Factory that chooses between the CLIP text-image space, a single ONNX
        /// text model and the deterministic fallback, based on what `models_dir` holds.
        pub fn create_embedder(
            models_dir: impl AsRef<Path>,
            downloads_dir: impl AsRef<Path>,
        ) -> Result<Arc<dyn Embedder>> {
            let models_dir = models_dir.as_ref();
            // If the onnx_runtime module was downloaded, point ort at the shared library.
            if let Some(runtime_path) = module_runtime_dylib(downloads_dir.as_ref(), "onnx_runtime") {
                let path_str = runtime_path.to_string_lossy().to_string();
                std::env::set_var("ORT_DYLIB_PATH", &path_str);
                tracing::info!("using downloaded ONNX Runtime library at {}", path_str);
            }

            match find_clip_artifacts(models_dir) {
                None => tracing::info!(
                    "No CLIP text/vision pair in {}; semantic search uses deterministic name \
                     vectors until the model modules are downloaded",
                    models_dir.display()
                ),
                Some(artifacts) => match ClipEmbedder::new(&artifacts) {
                    Ok(clip) => {
                        tracing::info!(
                            "using CLIP text-image embedder ({} dimensions) from {}",
                            clip.dimension(),
                            models_dir.display()
                        );
                        return Ok(Arc::new(clip));
                    }
                    Err(err) => tracing::warn!(
                        "CLIP models in {} cannot be used: {}; falling back to deterministic \
                         name vectors",
                        models_dir.display(),
                        err
                    ),
                },
            }
            Ok(Arc::new(FallbackEmbedder::new(DEFAULT_EMBEDDING_DIM)))
        }

        /// Collect files below `dir` up to `max_depth` levels deep.
        ///
        /// Module downloads are installed at `<dir>/<module>/<version>/…`, so a flat
        /// directory listing never sees them.
        fn collect_files(dir: &Path, max_depth: usize, out: &mut Vec<PathBuf>) {
            if max_depth == 0 {
                return;
            }
            let entries = match std::fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(_) => return,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let file_type = match entry.file_type() {
                    Ok(file_type) => file_type,
                    Err(_) => continue,
                };
                if file_type.is_dir() {
                    collect_files(&path, max_depth - 1, out);
                } else if file_type.is_file() {
                    out.push(path);
                }
            }
        }

        fn list_files(dir: &Path) -> Vec<PathBuf> {
            let mut files = Vec::new();
            collect_files(dir, 4, &mut files);
            files
        }

        /// The three artifacts a shared text-image space needs.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct ClipArtifacts {
            pub text_model: PathBuf,
            pub vision_model: PathBuf,
            pub tokenizer: PathBuf,
        }

        /// Locate a CLIP text encoder, vision encoder and tokenizer under `models_dir`.
        ///
        /// Returns `None` when any of the three is missing, because a half-installed
        /// CLIP pair cannot produce a shared space.
        pub fn find_clip_artifacts(models_dir: &Path) -> Option<ClipArtifacts> {
            let files = list_files(models_dir);
            let is_onnx = |p: &Path| p.extension().and_then(|e| e.to_str()) == Some("onnx");
            let name = |p: &Path| {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default()
            };
            let text_model = files
                .iter()
                .filter(|p| is_onnx(p))
                .filter(|p| name(p).contains("text"))
                .find(|p| !name(p).contains("tokenizer"))
                .cloned()?;
            let vision_model = files
                .iter()
                .filter(|p| is_onnx(p))
                .filter(|p| {
                    let n = name(p);
                    n.contains("visual") || n.contains("vision") || n.contains("image")
                })
                .next()
                .cloned()?;
            let mut tokenizers: Vec<PathBuf> = files
                .iter()
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
                .filter(|p| name(p).contains("tokenizer"))
                .cloned()
                .collect();
            tokenizers.sort_by_key(|p| if name(p) == "tokenizer.json" { 0 } else { 1 });
            let tokenizer = tokenizers.into_iter().next()?;
            Some(ClipArtifacts {
                text_model,
                vision_model,
                tokenizer,
            })
        }

        fn open_session(path: &Path) -> Result<Session> {
            Session::builder()
                .map_err(|e| anyhow!("failed to create ONNX session builder: {}", e))?
                .with_optimization_level(GraphOptimizationLevel::Level3)
                .map_err(|e| anyhow!("failed to set optimization level: {}", e))?
                .commit_from_file(path)
                .map_err(|e| anyhow!("failed to load ONNX model at {}: {}", path.display(), e))
        }

        fn input_names(session: &Session) -> Vec<String> {
            session.inputs().iter().map(|i| i.name().to_string()).collect()
        }

        /// CLIP ViT-B/32 embedder: text and images in one 512-dimensional space.
        ///
        /// Both encoders are the ONNX exports published for `clip-vit-base-patch32`;
        /// each already applies its projection, so their outputs are directly
        /// comparable after L2 normalization.
        pub struct ClipEmbedder {
            text_session: Mutex<Session>,
            vision_session: Mutex<Session>,
            text_inputs: Vec<String>,
            vision_inputs: Vec<String>,
            tokenizer: Tokenizer,
            dimension: usize,
        }

        impl ClipEmbedder {
            pub fn new(artifacts: &ClipArtifacts) -> Result<Self> {
                let text_session = open_session(&artifacts.text_model)?;
                let vision_session = open_session(&artifacts.vision_model)?;
                let tokenizer = Tokenizer::from_file(&artifacts.tokenizer)
                    .map_err(|e| anyhow!("failed to load CLIP tokenizer {}: {}", artifacts.tokenizer.display(), e))?;
                let mut embedder = Self {
                    text_inputs: input_names(&text_session),
                    vision_inputs: input_names(&vision_session),
                    text_session: Mutex::new(text_session),
                    vision_session: Mutex::new(vision_session),
                    tokenizer,
                    dimension: CLIP_EMBEDDING_DIM,
                };
                // Probe both encoders: a shared space only exists if the two sides
                // really produce the same number of dimensions.
                let text_dim = embedder
                    .run_text(&[String::from("mirage probe")])
                    .context("CLIP text encoder produced no output")?
                    .first()
                    .map(|v| v.len())
                    .unwrap_or_default();
                let blank = vec![0_f32; 3 * CLIP_IMAGE_SIZE * CLIP_IMAGE_SIZE];
                let vision_dim = embedder
                    .run_vision(vec![blank])
                    .context("CLIP vision encoder produced no output")?
                    .first()
                    .map(|v| v.len())
                    .unwrap_or_default();
                if text_dim == 0 || vision_dim == 0 {
                    return Err(anyhow!("CLIP encoders returned no vectors"));
                }
                if text_dim != vision_dim {
                    return Err(anyhow!(
                        "CLIP text encoder emits {} dimensions but the vision encoder emits {}; they are not a shared space",
                        text_dim,
                        vision_dim
                    ));
                }
                embedder.dimension = text_dim;
                tracing::info!(
                    "CLIP embedder loaded: text {} / vision {} from {} + {}",
                    text_dim,
                    vision_dim,
                    artifacts.text_model.display(),
                    artifacts.vision_model.display()
                );
                Ok(embedder)
            }

            fn encode(&self, text: &str) -> Result<(Vec<i64>, Vec<i64>)> {
                let encoding = self
                    .tokenizer
                    .encode(text, true)
                    .map_err(|e| anyhow!("CLIP tokenization failed: {}", e))?;
                let ids: Vec<i64> = encoding.get_ids().iter().map(|t| *t as i64).collect();
                Ok(clip_align_tokens(&ids))
            }

            fn run_text(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
                if texts.is_empty() {
                    return Ok(Vec::new());
                }
                let mut ids = Vec::with_capacity(texts.len() * CLIP_CONTEXT_LENGTH);
                let mut masks = Vec::with_capacity(texts.len() * CLIP_CONTEXT_LENGTH);
                for text in texts {
                    let (chunk_ids, chunk_mask) = self.encode(text)?;
                    ids.extend(chunk_ids);
                    masks.extend(chunk_mask);
                }
                let batch = texts.len();
                let shape = vec![batch, CLIP_CONTEXT_LENGTH];
                let mut inputs: Vec<(String, Value)> = Vec::new();
                if self.text_inputs.iter().any(|n| n == "input_ids") {
                    let tensor = Tensor::from_array((shape.clone(), ids))
                        .context("failed to create input_ids tensor")?;
                    inputs.push(("input_ids".to_string(), tensor.into()));
                }
                if self.text_inputs.iter().any(|n| n == "attention_mask") {
                    let tensor = Tensor::from_array((shape, masks))
                        .context("failed to create attention_mask tensor")?;
                    inputs.push(("attention_mask".to_string(), tensor.into()));
                }
                let mut session = self
                    .text_session
                    .lock()
                    .map_err(|e| anyhow!("failed to lock CLIP text session: {}", e))?;
                let outputs = session.run(inputs).context("CLIP text inference failed")?;
                let (_, value) = outputs
                    .iter()
                    .next()
                    .context("CLIP text encoder returned no outputs")?;
                let (_, data) = value
                    .try_extract_tensor::<f32>()
                    .context("failed to extract the CLIP text output")?;
                flatten_batch(data, batch)
            }

            fn run_vision(&self, pixels: Vec<Vec<f32>>) -> Result<Vec<Vec<f32>>> {
                if pixels.is_empty() {
                    return Ok(Vec::new());
                }
                let batch = pixels.len();
                let flat: Vec<f32> = pixels.into_iter().flatten().collect();
                let shape = vec![batch, 3, CLIP_IMAGE_SIZE, CLIP_IMAGE_SIZE];
                let name = self
                    .vision_inputs
                    .first()
                    .cloned()
                    .unwrap_or_else(|| String::from("pixel_values"));
                let tensor = Tensor::from_array((shape, flat))
                    .context("failed to create pixel_values tensor")?;
                let inputs: Vec<(String, Value)> = vec![(name, tensor.into())];
                let mut session = self
                    .vision_session
                    .lock()
                    .map_err(|e| anyhow!("failed to lock CLIP vision session: {}", e))?;
                let outputs = session.run(inputs).context("CLIP vision inference failed")?;
                let (_, value) = outputs
                    .iter()
                    .next()
                    .context("CLIP vision encoder returned no outputs")?;
                let (_, data) = value
                    .try_extract_tensor::<f32>()
                    .context("failed to extract the CLIP vision output")?;
                flatten_batch(data, batch)
            }

            /// Run one vision batch and write the vectors back to their original slots.
            fn flush_images(
                &self,
                ready: &mut Vec<(usize, Vec<f32>)>,
                results: &mut Vec<Option<Result<Vec<f32>>>>,
            ) {
                if ready.is_empty() {
                    return;
                }
                let batch = std::mem::take(ready);
                let indices: Vec<usize> = batch.iter().map(|(index, _)| *index).collect();
                let pixels: Vec<Vec<f32>> = batch.into_iter().map(|(_, pixels)| pixels).collect();
                match self.run_vision(pixels) {
                    Ok(vectors) => {
                        for (index, vector) in indices.into_iter().zip(vectors) {
                            results[index] = Some(Ok(vector));
                        }
                    }
                    Err(e) => {
                        for index in indices {
                            results[index] = Some(Err(anyhow!("CLIP vision batch failed: {}", e)));
                        }
                    }
                }
            }
        }

        /// Read an output tensor as one normalized vector per batch item.
        fn flatten_batch(data: &[f32], batch: usize) -> Result<Vec<Vec<f32>>> {
            if batch == 0 || data.len() % batch != 0 {
                return Err(anyhow!(
                    "output of {} floats is not divisible by batch size {}",
                    data.len(),
                    batch
                ));
            }
            let dim = data.len() / batch;
            Ok(data
                .chunks(dim)
                .map(|chunk| normalize(&mut chunk.to_vec()))
                .collect())
        }

        impl Embedder for ClipEmbedder {
            fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
                if text.trim().is_empty() {
                    return Ok(vec![0.0_f32; self.dimension]);
                }
                self.run_text(&[text.to_string()])?
                    .into_iter()
                    .next()
                    .context("CLIP text encoder returned no vector")
            }

            fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
                if texts.is_empty() {
                    return Ok(Vec::new());
                }
                let mut out = Vec::with_capacity(texts.len());
                let mut pending: Vec<String> = Vec::with_capacity(texts.len());
                for text in texts {
                    if text.trim().is_empty() {
                        out.push(vec![0.0_f32; self.dimension]);
                        continue;
                    }
                    pending.push(text.clone());
                    if pending.len() >= CLIP_TEXT_BATCH_CAP {
                        out.extend(self.run_text(&pending)?);
                        pending.clear();
                    }
                }
                if !pending.is_empty() {
                    out.extend(self.run_text(&pending)?);
                }
                Ok(out)
            }

            fn embed_texts_batched(&self, texts: &[String], memory_budget_mb: usize) -> Result<Vec<Vec<f32>>> {
                // The generic estimate ignores attention activations, which dominate for
                // a 77-token transformer; clamp it to what actually fits.
                let estimated = estimate_embedding_batch_size(
                    memory_budget_mb,
                    self.dimension(),
                    CLIP_CONTEXT_LENGTH,
                );
                let sub_batch = estimated.min(CLIP_TEXT_BATCH_CAP).max(1);
                let mut results = Vec::with_capacity(texts.len());
                for chunk in texts.chunks(sub_batch) {
                    results.extend(self.embed_texts(chunk)?);
                }
                Ok(results)
            }

            fn embed_image_file(&self, path: &Path) -> Result<Vec<f32>> {
                let pixels = load_clip_pixel_values(path)?;
                self.run_vision(vec![pixels])?
                    .into_iter()
                    .next()
                    .context("CLIP vision encoder returned no vector")
            }

            fn embed_image_files(&self, paths: &[PathBuf]) -> Vec<Result<Vec<f32>>> {
                let mut results: Vec<Option<Result<Vec<f32>>>> =
                    (0..paths.len()).map(|_| None).collect();
                let mut ready: Vec<(usize, Vec<f32>)> = Vec::new();
                for (index, path) in paths.iter().enumerate() {
                    match load_clip_pixel_values(path) {
                        Ok(pixels) => ready.push((index, pixels)),
                        Err(e) => results[index] = Some(Err(e)),
                    }
                    if ready.len() >= CLIP_IMAGE_BATCH_CAP {
                        self.flush_images(&mut ready, &mut results);
                    }
                }
                self.flush_images(&mut ready, &mut results);
                results
                    .into_iter()
                    .map(|r| r.unwrap_or_else(|| Err(anyhow!("image was not embedded"))))
                    .collect()
            }

            fn supports_images(&self) -> bool {
                true
            }

            fn dimension(&self) -> usize {
                self.dimension
            }
        }

        /// Look for the downloaded ONNX Runtime shared library in the module install
        /// directory, which is `<downloads_dir>/<module_id>/<version>/`.
        fn module_runtime_dylib(
            config_downloads_dir: &Path,
            module_id: &str,
        ) -> Option<PathBuf> {
            // `MIRAGE_DOWNLOADS_DIR` overrides the daemon's configured directory,
            // for tests and development machines that stage the module elsewhere.
            let base = std::env::var_os("MIRAGE_DOWNLOADS_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| config_downloads_dir.to_path_buf());
            let dir = base.join(module_id);
            if !dir.is_dir() {
                return None;
            }
            // The archive only materialises the versioned `.so` on Linux; the
            // `libonnxruntime.so` symlinks are not extracted, so match by prefix.
            let is_lib = |p: &PathBuf| {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if cfg!(target_os = "windows") {
                    name == "onnxruntime.dll"
                } else if cfg!(target_os = "macos") {
                    name == "libonnxruntime.dylib"
                } else {
                    name.starts_with("libonnxruntime.so")
                }
            };
            list_files(&dir).into_iter().find(|p| is_lib(p))
        }
    } else {
        /// Factory that always returns the deterministic fallback embedder when the `onnx`
        /// feature is disabled. The ONNX Runtime module can still be downloaded, but it cannot
        /// be used until the daemon is rebuilt with the `onnx` feature.
        pub fn create_embedder(
            _models_dir: impl AsRef<Path>,
            _downloads_dir: impl AsRef<Path>,
        ) -> Result<Arc<dyn Embedder>> {
            Ok(Arc::new(FallbackEmbedder::new(DEFAULT_EMBEDDING_DIM)))
        }
    }
}

/// Deterministic fallback embedder that produces stable pseudo-random vectors.
///
/// It is not a semantic model: two texts only match when their strings match. It
/// keeps the pipeline working before any model has been downloaded, and it never
/// claims to understand images.
pub struct FallbackEmbedder {
    dimension: usize,
}

impl FallbackEmbedder {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    fn vector_from_seed(&self, bytes: &[u8]) -> Vec<f32> {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let seed = <[u8; 32]>::try_from(&hasher.finalize()[..32]).unwrap();
        let rng = StdRng::from_seed(seed);
        let normal = Normal::new(0.0_f32, 1.0_f32).unwrap();
        let mut values: Vec<f32> = normal.sample_iter(rng).take(self.dimension).collect();
        normalize(&mut values)
    }
}

impl Embedder for FallbackEmbedder {
    fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(vec![0.0_f32; self.dimension]);
        }
        Ok(self.vector_from_seed(trimmed.as_bytes()))
    }

    fn embed_image_file(&self, path: &Path) -> Result<Vec<f32>> {
        Err(anyhow!(
            "no vision model is installed, so {} cannot be embedded from its contents; \
             download the CLIP modules to enable image search",
            path.display()
        ))
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn is_semantic(&self) -> bool {
        false
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

    #[test]
    fn fallback_refuses_images() {
        let embedder = FallbackEmbedder::new(8);
        assert!(!embedder.supports_images());
        assert!(embedder
            .embed_image_file(Path::new("/tmp/none.png"))
            .is_err());
    }

    #[test]
    fn estimate_batch_size_is_positive() {
        let batch = super::estimate_embedding_batch_size(3072, 384, 128);
        assert!(batch > 0);
    }

    #[test]
    fn estimate_batch_size_clamps_to_one() {
        let batch = super::estimate_embedding_batch_size(0, 384, 128);
        assert_eq!(batch, 1);
    }

    #[test]
    fn image_batch_size_stays_within_the_caps() {
        assert_eq!(estimate_image_batch_size(0), 1);
        assert_eq!(estimate_image_batch_size(3072), CLIP_IMAGE_BATCH_CAP);
    }

    #[test]
    fn clip_alignment_pads_to_the_context_length() {
        let (ids, mask) = clip_align_tokens(&[49406, 21, 49407]);
        assert_eq!(ids.len(), CLIP_CONTEXT_LENGTH);
        assert_eq!(mask.len(), CLIP_CONTEXT_LENGTH);
        assert_eq!(&ids[..3], &[49406, 21, 49407][..]);
        assert_eq!(ids[3], CLIP_PAD_TOKEN);
        assert_eq!(mask.iter().filter(|m| **m == 1).count(), 3);
    }

    #[test]
    fn clip_alignment_keeps_the_end_token_when_truncating() {
        let ids: Vec<i64> = (0..200_i64).collect();
        let (aligned, mask) = clip_align_tokens(&ids);
        assert_eq!(aligned.len(), CLIP_CONTEXT_LENGTH);
        assert_eq!(aligned[0], 0);
        assert_eq!(
            *aligned.last().unwrap(),
            199,
            "the end-of-text token survives"
        );
        assert!(
            mask.iter().all(|m| *m == 1),
            "a full context needs no padding"
        );
    }

    #[test]
    fn pixel_values_are_normalized_chw_for_one_crop() {
        let image = image::RgbImage::from_pixel(512, 256, image::Rgb([255, 0, 128]));
        let pixels = clip_pixel_values(&image);
        assert_eq!(pixels.len(), 3 * CLIP_IMAGE_SIZE * CLIP_IMAGE_SIZE);
        let plane = CLIP_IMAGE_SIZE * CLIP_IMAGE_SIZE;
        // Red channel: 255 raw, normalized with the CLIP mean and standard deviation.
        let red = pixels[plane - 1];
        let expected_red = (255.0 / 255.0 - CLIP_MEAN[0]) / CLIP_STD[0];
        assert!(
            (red - expected_red).abs() < 1e-4,
            "red was {red}, expected {expected_red}"
        );
        // Blue channel: 128 raw.
        let blue = pixels[2 * plane + plane - 1];
        let expected_blue = (128.0 / 255.0 - CLIP_MEAN[2]) / CLIP_STD[2];
        assert!(
            (blue - expected_blue).abs() < 1e-4,
            "blue was {blue}, expected {expected_blue}"
        );
    }

    #[test]
    fn pixel_values_keep_the_aspect_ratio_and_center_crop() {
        // A 448x224 image scales to 448x224 then crops to the central 224x224.
        let mut img = image::RgbImage::new(448, 224);
        for (x, _y, pixel) in img.enumerate_pixels_mut() {
            *pixel = if x < 224 {
                image::Rgb([0, 0, 0])
            } else {
                image::Rgb([255, 255, 255])
            };
        }
        let pixels = clip_pixel_values(&img);
        let plane = CLIP_IMAGE_SIZE * CLIP_IMAGE_SIZE;
        // The crop starts at x = 112, so the left half of the crop is still black.
        let left = pixels[100];
        let right = pixels[plane - 100];
        assert!(left < 0.0, "left side should be below the mean");
        assert!(right > 0.0, "right side should be above the mean");
    }

    #[cfg(feature = "onnx")]
    #[test]
    fn clip_artifacts_need_all_three_files() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(find_clip_artifacts(dir.path()).is_none());

        let models = dir.path().join("clip_text_encoder").join("1.0.0");
        std::fs::create_dir_all(&models).unwrap();
        std::fs::File::create(models.join("text_model_int8.onnx")).unwrap();
        // Still missing the vision encoder and the tokenizer.
        assert!(find_clip_artifacts(dir.path()).is_none());

        let vision = dir.path().join("clip_vision_encoder").join("1.0.0");
        std::fs::create_dir_all(&vision).unwrap();
        std::fs::File::create(vision.join("vision_model_int8.onnx")).unwrap();
        let tokenizer_dir = dir.path().join("clip_tokenizer").join("1.0.0");
        std::fs::create_dir_all(&tokenizer_dir).unwrap();
        std::fs::File::create(tokenizer_dir.join("clip_tokenizer.json")).unwrap();

        let artifacts = find_clip_artifacts(dir.path()).expect("a nested CLIP pair is discovered");
        assert!(artifacts.text_model.ends_with("text_model_int8.onnx"));
        assert!(artifacts.vision_model.ends_with("vision_model_int8.onnx"));
    }
}
