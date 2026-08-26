from fastapi import APIRouter

from app.db import get_latest_version, get_or_create_table

router = APIRouter(prefix="/sync", tags=["sync"])


@router.get("/delta")
def get_delta_index(version: int = 0):
    """Return the delta of new record ids since *version*.

    This is an MVP placeholder: it queries LanceDB for records whose
    ``version`` is greater than the client's last known version and
    returns their ids. Real ``.lance`` delta streaming will be added later.
    """
    table = get_or_create_table()
    current_version = get_latest_version()

    if version >= current_version:
        return {
            "version": current_version,
            "new_record_ids": [],
            "removed_record_ids": [],
            "description": "no delta",
        }

    arrow_table = table.to_arrow()
    version_array = arrow_table.column("version").to_pylist()
    id_array = arrow_table.column("id").to_pylist()
    new_record_ids = [
        file_id for file_id, record_version in zip(id_array, version_array)
        if record_version > version
    ]

    return {
        "version": current_version,
        "client_version": version,
        "new_record_ids": new_record_ids,
        "removed_record_ids": [],
        "description": f"records with version > {version}",
    }
