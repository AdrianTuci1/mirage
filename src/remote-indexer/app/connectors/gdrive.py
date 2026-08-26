from collections.abc import Iterator

from app.connectors.base import SourceConnector
from app.models.file_entry import FileEntry


class GoogleDriveConnector(SourceConnector):
    """Stub connector for Google Drive (to be implemented)."""

    def scan(self) -> Iterator[FileEntry]:
        raise NotImplementedError("Google Drive connector not implemented yet")

    def read(self, path: str) -> bytes:
        raise NotImplementedError("Google Drive connector not implemented yet")
