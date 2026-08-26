import json

import pytest
from fastapi.testclient import TestClient

from app import config
from app.db import build_record, bump_version, get_or_create_table
from app.main import app


client = TestClient(app)


@pytest.fixture
def known_passkey(monkeypatch):
    """Set a predictable passkey for sync endpoint tests."""
    monkeypatch.setattr(config.settings, "secret_key", "test-secret-key")


def test_health_check():
    response = client.get("/health")
    assert response.status_code == 200
    assert response.json() == {"status": "ok"}


def test_sync_delta_returns_records(
    temp_index_dir, known_passkey, monkeypatch
):
    monkeypatch.setattr(config.settings, "lancedb_uri", temp_index_dir)

    table = get_or_create_table()
    record = build_record(
        file_id="abc123",
        relative_path="photos/cat.jpg",
        source_type="local",
        vector=[0.1, 0.2, 0.3],
        version=1,
    )
    table.add([record])
    bump_version()

    response = client.get(
        "/sync/delta?version=0",
        headers={"Authorization": "Bearer test-secret-key"},
    )

    assert response.status_code == 200
    assert response.headers["X-Latest-Version"] == "1"
    assert response.headers["content-type"] == "application/x-ndjson"

    lines = [line for line in response.text.strip().split("\n") if line]
    assert len(lines) == 1

    payload = json.loads(lines[0])
    assert payload["id"] == "abc123"
    assert payload["relative_path"] == "photos/cat.jpg"
    assert payload["source_type"] == "local"
    assert payload["version"] == 1
    assert payload["updated_at"].endswith("Z")


def test_sync_delta_empty_when_up_to_date(
    temp_index_dir, known_passkey, monkeypatch
):
    monkeypatch.setattr(config.settings, "lancedb_uri", temp_index_dir)

    table = get_or_create_table()
    record = build_record(
        file_id="abc123",
        relative_path="photos/cat.jpg",
        source_type="local",
        vector=[0.1, 0.2, 0.3],
        version=1,
    )
    table.add([record])
    bump_version()

    response = client.get(
        "/sync/delta?version=1",
        headers={"Authorization": "Bearer test-secret-key"},
    )

    assert response.status_code == 200
    assert response.headers["X-Latest-Version"] == "1"
    assert response.text == ""


def test_sync_delta_rejects_missing_auth(temp_index_dir, monkeypatch):
    monkeypatch.setattr(config.settings, "lancedb_uri", temp_index_dir)

    response = client.get("/sync/delta?version=0")
    assert response.status_code == 401
