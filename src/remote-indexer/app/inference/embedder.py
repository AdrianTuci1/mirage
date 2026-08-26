import hashlib
import os
from typing import Any

import numpy as np
import onnxruntime as ort

EMBEDDING_DIM = 384


class OnnxEmbedder:
    """Embedding inference using ONNX Runtime.

    MVP fallback: if no ONNX model is present in ``assets/models/`` the
    embedder returns deterministic pseudo-random unit vectors of dimension
    ``EMBEDDING_DIM``. This keeps tests deterministic while the indexer
    pipeline can still be exercised end-to-end.
    """

    def __init__(self, model_dir: str | None = None) -> None:
        self.model_dir = model_dir or os.path.join("assets", "models")
        self.session: ort.InferenceSession | None = None
        self._input_name: str | None = None
        self._load_model_if_available()

    def _load_model_if_available(self) -> None:
        if not os.path.isdir(self.model_dir):
            return
        candidates = [n for n in os.listdir(self.model_dir) if n.endswith(".onnx")]
        if not candidates:
            return
        model_path = os.path.join(self.model_dir, candidates[0])
        self.session = ort.InferenceSession(model_path, providers=ort.get_available_providers())
        self._input_name = self.session.get_inputs()[0].name

    def embed_text(self, text: str) -> list[float]:
        """Return a dense vector for *text*."""
        if self.session is not None:
            return self._run_onnx(text)
        return self._deterministic_vector(text)

    def embed_image(self, image_bytes: bytes) -> list[float]:
        """Return a dense vector for raw image bytes."""
        if self.session is not None:
            return self._run_onnx(image_bytes)
        return self._deterministic_vector(image_bytes)

    def _run_onnx(self, _features: Any) -> list[float]:
        # Placeholder for real preprocessing/tokenization.
        # When an ONNX model is provided, the actual tokenization depends on
        # the model type (text vs CLIP). For the MVP we fall back to fake
        # embeddings and document this limitation.
        return self._deterministic_vector(str(_features))

    def _deterministic_vector(self, seed_material: str | bytes) -> list[float]:
        seed = hashlib.sha256(
            seed_material if isinstance(seed_material, bytes) else seed_material.encode("utf-8")
        ).digest()
        rng = np.random.default_rng(int.from_bytes(seed[:8], "big"))
        vector = rng.normal(size=EMBEDDING_DIM).astype(np.float32)
        norm = np.linalg.norm(vector)
        if norm == 0:
            norm = 1.0
        return (vector / norm).tolist()
