from app.inference.embedder import EMBEDDING_DIM, OnnxEmbedder

_embedder_instance: OnnxEmbedder | None = None


def get_embedder(model_dir: str | None = None) -> OnnxEmbedder:
    """Return a singleton :class:`OnnxEmbedder`."""
    global _embedder_instance
    if _embedder_instance is None:
        _embedder_instance = OnnxEmbedder(model_dir)
    return _embedder_instance


__all__ = ["EMBEDDING_DIM", "OnnxEmbedder", "get_embedder"]
