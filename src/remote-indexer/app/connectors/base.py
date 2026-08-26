from abc import ABC, abstractmethod
from collections.abc import Iterator

from app.models.file_entry import FileEntry


class SourceConnector(ABC):
    """Abstract base class for read-only storage connectors."""

    @abstractmethod
    def scan(self) -> Iterator[FileEntry]:
        """Yield all files reachable through this connector."""
        raise NotImplementedError

    @abstractmethod
    def read(self, path: str) -> bytes:
        """Return the raw bytes of the file at *path*.

        *path* is the connector-relative path (i.e. FileEntry.relative_path).
        """
        raise NotImplementedError
