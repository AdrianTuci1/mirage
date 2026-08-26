from dataclasses import dataclass
from datetime import datetime


@dataclass
class FileEntry:
    """Represents a single file discovered by a storage connector."""

    relative_path: str
    absolute_path: str
    source_type: str
    size: int
    mtime: datetime
    unique_hash: str
