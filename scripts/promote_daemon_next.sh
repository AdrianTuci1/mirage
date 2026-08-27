#!/usr/bin/env bash
# Promote src/daemon_next to src/daemon after review.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DAEMON_DIR="$PROJECT_DIR/src/daemon"
NEXT_DIR="$PROJECT_DIR/src/daemon_next"

if [ ! -d "$NEXT_DIR" ]; then
    echo "Error: $NEXT_DIR does not exist." >&2
    exit 1
fi

if [ -d "$DAEMON_DIR" ]; then
    echo "Removing existing $DAEMON_DIR ..."
    rm -rf "$DAEMON_DIR"
fi

echo "Promoting $NEXT_DIR -> $DAEMON_DIR ..."
mv "$NEXT_DIR" "$DAEMON_DIR"

echo "Running cargo test in $DAEMON_DIR ..."
cd "$DAEMON_DIR"
cargo test

echo "Done. src/daemon now contains the updated daemon implementation."
