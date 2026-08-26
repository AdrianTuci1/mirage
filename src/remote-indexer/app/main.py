from fastapi import FastAPI

from app.api import sync

app = FastAPI(
    title="Mirage Remote Indexer",
    description="Dockerized semantic indexing service for Mirage.",
    version="0.1.0",
)

app.include_router(sync.router)


@app.get("/health")
def health_check():
    """Health check endpoint."""
    return {"status": "ok"}
