import os
import tempfile

import pytest

from app import config
from app.connectors.local import LocalConnector
from app.db import get_latest_version, get_or_create_table
from app.inference import get_embedder
from app.pipeline.indexer import Indexer


@pytest.fixture
def temp_source_dir():
    """Provide a temporary directory populated with sample files."""
    with tempfile.TemporaryDirectory() as tmpdir:
        txt_path = os.path.join(tmpdir, "hello.txt")
        with open(txt_path, "w", encoding="utf-8") as f:
            f.write("Mirage local-first semantic search.")

        png_path = os.path.join(tmpdir, "image.png")
        # Minimal valid 1x1 PNG.
        with open(png_path, "wb") as f:
            f.write(
                b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01"
                b"\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0c"
                b"IDATx\x9cc\xf8\x00\x00\x00\x01\x01\x00\x05\xfe\x02\xfe\x00\x00"
                b"\x00\x00IEND\xaeB`\x82"
            )

        yield tmpdir


def test_indexer_creates_records_and_bumps_version(
    temp_source_dir, temp_index_dir, monkeypatch
):
    monkeypatch.setattr(config.settings, "lancedb_uri", temp_index_dir)
    monkeypatch.setattr(config.settings, "source_path", temp_source_dir)

    connector = LocalConnector(temp_source_dir)
    indexer = Indexer()
    summary = indexer.run_indexing_pipeline(connector)

    assert summary["scanned"] == 2
    assert summary["added"] == 2
    assert summary["skipped"] == 0
    assert summary["version"] == 1
    assert get_latest_version() == 1

    table = get_or_create_table()
    rows = table.search().limit(10).to_list()
    assert len(rows) == 2

    relative_paths = {row["relative_path"] for row in rows}
    assert relative_paths == {"hello.txt", "image.png"}

    for row in rows:
        assert row["source_type"] == "local"
        assert row["version"] == 1
        assert len(row["vector"]) == 384


def test_indexer_incremental_run_skips_unchanged_files(
    temp_source_dir, temp_index_dir, monkeypatch
):
    monkeypatch.setattr(config.settings, "lancedb_uri", temp_index_dir)
    monkeypatch.setattr(config.settings, "source_path", temp_source_dir)

    connector = LocalConnector(temp_source_dir)
    indexer = Indexer()

    first_summary = indexer.run_indexing_pipeline(connector, incremental=True)
    assert first_summary["added"] == 2

    second_summary = indexer.run_indexing_pipeline(connector, incremental=True)
    assert second_summary["scanned"] == 2
    assert second_summary["added"] == 0
    assert second_summary["skipped"] == 2
    assert second_summary["version"] == 2
    assert get_latest_version() == 2


def test_indexer_embeds_deterministically_for_missing_model(
    temp_source_dir, temp_index_dir, monkeypatch
):
    monkeypatch.setattr(config.settings, "lancedb_uri", temp_index_dir)

    embedder = get_embedder(model_dir="/nonexistent/models")
    vector_a = embedder.embed_text("hello")
    vector_b = embedder.embed_text("hello")
    assert vector_a == vector_b
    assert len(vector_a) == 384
