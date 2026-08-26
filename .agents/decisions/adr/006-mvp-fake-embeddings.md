# ADR 006: MVP Fake Embeddings for Remote Indexer

## Status

Accepted

## Context

Task T1.4 requires integrating ONNX Runtime for embedding inference. A production
implementation would download a small text-embedding model (e.g.
all-MiniLM-L6-v2, ~80 MB, ONNX Community License) and optionally a CLIP-based
image-embedding model. However, for the MVP milestone we need:

- A working indexing pipeline that can be exercised end-to-end without requiring
  external model downloads during setup or CI.
- Deterministic, reproducible vectors so tests are stable.
- A fixed embedding dimension (384) that matches the intended text-embedding
  model and keeps LanceDB record schemas stable.

## Decision

Use a deterministic pseudo-random vector generator as the MVP embedding
fallback. `OnnxEmbedder` will still load and run an ONNX model if one is
present in `assets/models/`, but when no model is available it returns unit
vectors seeded from the input content hash.

The fallback is implemented in:
`src/remote-indexer/app/inference/embedder.py`

## Consequences

- The indexer pipeline (scan → extract → embed → store) can be tested
  immediately without downloading models.
- Vector search quality is not meaningful in MVP; similarity scores are
  arbitrary. This is acceptable because T1.4 only verifies the pipeline
  wiring.
- The embedding dimension (384) and interface (`embed_text`, `embed_image`)
  remain unchanged when a real ONNX model replaces the fallback.
- Future work: add `assets/models/` download script, tokenizer/preprocessing,
  and switch `OnnxEmbedder._run_onnx` to real inference.

## Note on model download

No large models are downloaded automatically. When a real model is added, its
size and license must be documented in the download script or `assets/README.md`.
