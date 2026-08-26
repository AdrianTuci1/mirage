import os


class Settings:
    """Application settings loaded from environment variables."""

    vault_name: str = os.environ.get("VAULT_NAME", "Company-NAS")
    sync_interval_sec: int = int(os.environ.get("SYNC_INTERVAL_SEC", "60"))
    max_cpu_threads: int = int(os.environ.get("MAX_CPU_THREADS", "4"))
    max_ram_mb: int = int(os.environ.get("MAX_RAM_MB", "4096"))
    secret_key: str = os.environ.get("SECRET_KEY", "generate_on_first_run")
    # Default to local ./data/index for development; override in Docker via env.
    lancedb_uri: str = os.environ.get("LANCEDB_URI", "./data/index")
    source_path: str = os.environ.get("SOURCE_PATH", "./data/source")


settings = Settings()
