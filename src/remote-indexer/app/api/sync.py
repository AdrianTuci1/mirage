from fastapi import APIRouter

router = APIRouter(prefix="/sync", tags=["sync"])


@router.get("/delta")
def get_delta_index(version: int = 0):
    """Placeholder delta sync endpoint.

    Returns the requested version and an empty list of delta files.
    Full implementation will stream .lance delta files based on the
    client's last known version.
    """
    return {"version": version, "files": []}
