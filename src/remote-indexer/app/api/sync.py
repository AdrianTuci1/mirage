import json

from fastapi import APIRouter, Depends, HTTPException, Response, Security, status
from fastapi.responses import StreamingResponse
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer

from app.config import settings
from app.db import get_latest_version, get_records_since_version

router = APIRouter(prefix="/sync", tags=["sync"])
security = HTTPBearer()


def verify_passkey(
    credentials: HTTPAuthorizationCredentials = Security(security),
) -> str:
    """Verify the Bearer token matches the configured secret key."""
    token = credentials.credentials
    if token != settings.secret_key:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid or missing passkey",
        )
    return token


def _ndjson_stream(version: int):
    """Yield records with version > *version* as JSON Lines."""
    for record in get_records_since_version(version):
        yield json.dumps(record, ensure_ascii=False) + "\n"


@router.get("/delta")
def get_delta_index(version: int = 0, passkey: str = Depends(verify_passkey)):
    """Stream index records added after *version* as NDJSON.

    Returns ``application/x-ndjson`` with one JSON object per line. The
    response includes an ``X-Latest-Version`` header containing the server's
    current index version. When the client is already up to date an empty body
    is returned.
    """
    current_version = get_latest_version()

    if version >= current_version:
        return Response(
            status_code=200,
            headers={"X-Latest-Version": str(current_version)},
        )

    return StreamingResponse(
        _ndjson_stream(version),
        media_type="application/x-ndjson",
        headers={"X-Latest-Version": str(current_version)},
    )
