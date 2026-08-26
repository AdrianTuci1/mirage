import os
import tempfile

import pytest


@pytest.fixture
def temp_index_dir():
    """Provide a temporary directory for LanceDB index tests."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield tmpdir


@pytest.fixture(autouse=True)
def clean_env():
    """Isolate environment variables for each test."""
    original = os.environ.copy()
    yield
    os.environ.clear()
    os.environ.update(original)
