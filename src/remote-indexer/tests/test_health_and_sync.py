from fastapi.testclient import TestClient

from app.main import app


client = TestClient(app)


def test_health_check():
    response = client.get("/health")
    assert response.status_code == 200
    assert response.json() == {"status": "ok"}


def test_sync_delta_placeholder():
    response = client.get("/sync/delta?version=0")
    assert response.status_code == 200
    body = response.json()
    assert body["version"] == 0
    assert body["files"] == []
