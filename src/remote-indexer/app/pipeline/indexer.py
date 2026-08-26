from collections.abc import Sequence
from datetime import datetime, timezone

from app.connectors.base import SourceConnector
from app.db import build_record, get_or_create_table
from app.inference import get_embedder
from app.models.file_entry import FileEntry
from app.pipeline import feature_extractor
from app.pipeline.versioning import bump_version, get_latest_version


class Indexer:
    """Orchestrates scan, feature extraction, embedding and storage."""

    def __init__(self) -> None:
        self.embedder = get_embedder()

    def run_indexing_pipeline(
        self,
        source_connector: SourceConnector,
        incremental: bool = True,
    ) -> dict:
        """Index all files reachable by *source_connector*.

        Returns a summary with the new version, added/removed/skipped counts.
        """
        table = get_or_create_table()
        version = get_latest_version()

        scanned_entries = list(source_connector.scan())
        scanned_ids = {entry.unique_hash for entry in scanned_entries}

        existing_ids: set[str] = set()
        if incremental:
            existing_ids = set(table.to_arrow().column("id").to_pylist())

        records: list[dict] = []
        skipped = 0
        for entry in scanned_entries:
            if incremental and entry.unique_hash in existing_ids:
                skipped += 1
                continue

            record = self._build_record_for_entry(entry, source_connector, version + 1)
            if record is not None:
                records.append(record)

        removed_ids: list[str] = []
        if incremental:
            removed_ids = sorted(existing_ids - scanned_ids)

        if removed_ids:
            table.delete(f"id IN ({_id_list_literal(removed_ids)})")

        if records:
            table.add(records)

        new_version = bump_version()

        return {
            "version": new_version,
            "added": len(records),
            "skipped": skipped,
            "removed": len(removed_ids),
            "scanned": len(scanned_entries),
        }

    def _build_record_for_entry(
        self,
        entry: FileEntry,
        connector: SourceConnector,
        version: int,
    ) -> dict | None:
        text = feature_extractor.extract_text(entry, connector)
        if text is not None:
            vector = self.embedder.embed_text(text)
        else:
            image_bytes = feature_extractor.extract_image_bytes(entry, connector)
            if image_bytes is None:
                return None
            vector = self.embedder.embed_image(image_bytes)

        return build_record(
            file_id=entry.unique_hash,
            relative_path=entry.relative_path,
            source_type=entry.source_type,
            vector=vector,
            updated_at=datetime.now(timezone.utc),
            version=version,
        )


def _id_list_literal(ids: Sequence[str]) -> str:
    """Build a LanceDB literal list of string ids."""
    if not ids:
        return "('')"
    quoted = ", ".join("'{}'".format(id_.replace("'", "''")) for id_ in ids)
    return f"({quoted})"
