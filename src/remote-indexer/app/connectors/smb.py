from collections.abc import Iterator

from app.connectors.base import SourceConnector
from app.models.file_entry import FileEntry


class SmbConnector(SourceConnector):
    """Stub connector for NAS/SMB (to be implemented)."""

    def scan(self) -> Iterator[FileEntry]:
        raise NotImplementedError("SMB/NAS connector not implemented yet")

    def read(self, path: str) -> bytes:
        raise NotImplementedError("SMB/NAS connector not implemented yet")
