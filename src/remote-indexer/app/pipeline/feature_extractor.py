from pathlib import Path

from app.connectors.base import SourceConnector
from app.models.file_entry import FileEntry

TEXT_EXTENSIONS = {".txt", ".md", ".markdown"}
IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".gif", ".bmp", ".webp"}
PDF_EXTENSIONS = {".pdf"}


def extract_text(entry: FileEntry, connector: SourceConnector) -> str | None:
    """Return plain text content for supported document types, else ``None``."""
    ext = Path(entry.relative_path).suffix.lower()

    if ext in TEXT_EXTENSIONS:
        data = connector.read(entry.relative_path)
        return data.decode("utf-8", errors="replace")

    if ext in PDF_EXTENSIONS:
        return _extract_pdf_text(entry, connector)

    return None


def extract_image_bytes(entry: FileEntry, connector: SourceConnector) -> bytes | None:
    """Return raw image bytes for supported image types, else ``None``."""
    ext = Path(entry.relative_path).suffix.lower()
    if ext in IMAGE_EXTENSIONS:
        return connector.read(entry.relative_path)
    return None


def _extract_pdf_text(entry: FileEntry, connector: SourceConnector) -> str | None:
    try:
        import fitz  # noqa: PLC0415
    except ImportError:  # pragma: no cover
        return None

    data = connector.read(entry.relative_path)
    try:
        doc = fitz.open(stream=data, filetype="pdf")
        return "\n".join(page.get_text() for page in doc)
    except Exception:  # noqa: BLE001
        return None
