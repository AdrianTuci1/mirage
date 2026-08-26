import hashlib
import os
from collections.abc import Iterator
from datetime import datetime, timezone

from app.connectors.base import SourceConnector
from app.models.file_entry import FileEntry


def _compute_unique_hash(root: str, relative_path: str, mtime: float, size: int) -> str:
    """Deterministic hash identifying a file across indexing runs."""
    digest = hashlib.blake2b(digest_size=16)
    digest.update(os.path.basename(root).encode("utf-8"))
    digest.update(relative_path.encode("utf-8"))
    digest.update(str(mtime).encode("utf-8"))
    digest.update(str(size).encode("utf-8"))
    return digest.hexdigest()


class LocalConnector(SourceConnector):
    """Read-only connector for a local directory tree."""

    def __init__(self, root: str) -> None:
        self.root = os.path.abspath(root)

    def scan(self) -> Iterator[FileEntry]:
        for dirpath, _dirs, filenames in os.walk(self.root):
            for filename in filenames:
                absolute_path = os.path.join(dirpath, filename)
                stat = os.stat(absolute_path)
                relative_path = os.path.relpath(absolute_path, self.root)
                yield FileEntry(
                    relative_path=relative_path,
                    absolute_path=absolute_path,
                    source_type="local",
                    size=stat.st_size,
                    mtime=datetime.fromtimestamp(stat.st_mtime, tz=timezone.utc),
                    unique_hash=_compute_unique_hash(
                        self.root, relative_path, stat.st_mtime, stat.st_size
                    ),
                )

    def read(self, path: str) -> bytes:
        absolute_path = os.path.join(self.root, path)
        if not os.path.abspath(absolute_path).startswith(self.root):
            raise ValueError("path escapes connector root")
        with open(absolute_path, "rb") as f:
            return f.read()
