"""Index versioning helpers.

The canonical version is persisted in a JSON file next to the LanceDB
directory and exposed by :mod:`app.db`. This module provides a pipeline-level
wrapper that can be extended with run metadata later.
"""

from app.db import bump_version as db_bump_version
from app.db import get_latest_version as db_get_latest_version


def get_latest_version() -> int:
    """Return the latest committed index version."""
    return db_get_latest_version()


def bump_version() -> int:
    """Increment the index version after a successful indexing run."""
    return db_bump_version()
