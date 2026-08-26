import json
import os
from datetime import datetime, timezone
from typing import Any

import lancedb
import pyarrow as pa

from app.config import settings

# LanceDB record schema per technical spec:
# {
#   "id": "string (unique hash)",
#   "relative_path": "string",
#   "source_type": "enum: nas | dropbox | s3 | gdrive | local",
#   "vector": "[float]",
#   "updated_at": "timestamp",
#   "version": "int64"
# }
RECORD_SCHEMA = pa.schema([
    pa.field("id", pa.string(), nullable=False),
    pa.field("relative_path", pa.string(), nullable=False),
    pa.field(
        "source_type",
        pa.dictionary(pa.int8(), pa.string()),
        nullable=False,
    ),
    pa.field("vector", pa.list_(pa.float32()), nullable=False),
    pa.field("updated_at", pa.timestamp("us"), nullable=False),
    pa.field("version", pa.int64(), nullable=False),
])

TABLE_NAME = "mirage_index"
VERSION_FILE = "version.json"


def get_db() -> lancedb.DBConnection:
    """Return a LanceDB connection using the configured URI."""
    os.makedirs(settings.lancedb_uri, exist_ok=True)
    return lancedb.connect(settings.lancedb_uri)


def get_or_create_table() -> lancedb.table.Table:
    """Open or create the index table with the defined record schema."""
    db = get_db()
    existing = db.list_tables()
    if TABLE_NAME in existing:
        return db.open_table(TABLE_NAME)
    return db.create_table(TABLE_NAME, schema=RECORD_SCHEMA, exist_ok=True)


def build_record(
    file_id: str,
    relative_path: str,
    source_type: str,
    vector: list[float],
    updated_at: datetime | None = None,
    version: int = 1,
) -> dict[str, Any]:
    """Build a record matching the LanceDB record schema."""
    return {
        "id": file_id,
        "relative_path": relative_path,
        "source_type": source_type,
        "vector": vector,
        "updated_at": updated_at or datetime.now(timezone.utc),
        "version": version,
    }


def version_file_path() -> str:
    """Return the path to the version JSON file stored next to LanceDB."""
    return os.path.join(settings.lancedb_uri, VERSION_FILE)


def get_latest_version() -> int:
    """Return the latest committed index version, or 0 if none exists."""
    path = version_file_path()
    if not os.path.exists(path):
        return 0
    try:
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
        return int(data.get("version", 0))
    except (json.JSONDecodeError, ValueError, OSError):
        return 0


def bump_version() -> int:
    """Increment and persist the index version. Returns the new version."""
    new_version = get_latest_version() + 1
    os.makedirs(settings.lancedb_uri, exist_ok=True)
    with open(version_file_path(), "w", encoding="utf-8") as f:
        json.dump(
            {"version": new_version, "updated_at": datetime.now(timezone.utc).isoformat()},
            f,
        )
    return new_version
