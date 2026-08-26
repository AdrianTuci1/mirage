from collections.abc import Iterator

from app.connectors.base import SourceConnector
from app.models.file_entry import FileEntry


class DropboxConnector(SourceConnector):
    """Stub connector for Dropbox (to be implemented)."""

    def scan(self) -> Iterator[FileEntry]:
        raise NotImplementedError("Dropbox connector not implemented yet")

    def read(self, path: str) -> bytes:
        raise NotImplementedError("Dropbox connector not implemented yet")
