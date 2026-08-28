#!/usr/bin/env bash
set -euo pipefail

# Package Mirage for Linux (DEB).
# Run this from a Linux machine with Rust, JDK 21 and Gradle available.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DAEMON_DIR="${PROJECT_ROOT}/src/daemon_next"
KMP_DIR="${PROJECT_ROOT}/src/client-kmp"
RESOURCES_DIR="${KMP_DIR}/package-resources"

cd "${PROJECT_ROOT}"
mkdir -p "${RESOURCES_DIR}/linux"

echo "[Linux] Building Rust daemon and CLI..."
cd "${DAEMON_DIR}"
cargo build --release --bin mirage-daemon --bin mirage

echo "[Linux] Staging binaries for Compose packaging..."
cp "${DAEMON_DIR}/target/release/mirage-daemon" "${RESOURCES_DIR}/linux/mirage-daemon"
cp "${DAEMON_DIR}/target/release/mirage" "${RESOURCES_DIR}/linux/mirage"
chmod +x "${RESOURCES_DIR}/linux/mirage-daemon" "${RESOURCES_DIR}/linux/mirage"

echo "[Linux] Packaging DEB via Gradle..."
cd "${KMP_DIR}"
./gradlew packageDeb

DEB_FILE=$(find "${KMP_DIR}/build/compose/binaries/main-release/deb" -maxdepth 1 -name "*.deb" -print -quit 2>/dev/null || true)
if [[ -n "${DEB_FILE}" && -f "${DEB_FILE}" ]]; then
    echo "[Linux] DEB installer: ${DEB_FILE}"
fi

echo "[Linux] Install with: sudo dpkg -i <deb-file>"
