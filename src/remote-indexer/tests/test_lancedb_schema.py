from app import config
from app.db import build_record, get_or_create_table, RECORD_SCHEMA


def test_record_schema_matches_spec():
    fields = {f.name: f for f in RECORD_SCHEMA}
    assert "id" in fields
    assert "relative_path" in fields
    assert "source_type" in fields
    assert "vector" in fields
    assert "updated_at" in fields


def test_table_creation_and_record_insert(temp_index_dir, monkeypatch):
    # Patch the module-level settings so the test uses the temp directory.
    monkeypatch.setattr(config.settings, "lancedb_uri", temp_index_dir)

    table = get_or_create_table()
    assert table.name == "mirage_index"

    record = build_record(
        file_id="abc123",
        relative_path="photos/cat.jpg",
        source_type="nas",
        vector=[0.1, 0.2, 0.3],
    )
    table.add([record])

    rows = table.search().limit(10).to_list()
    assert len(rows) == 1
    assert rows[0]["relative_path"] == "photos/cat.jpg"
    assert rows[0]["source_type"] == "nas"
