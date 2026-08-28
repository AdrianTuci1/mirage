#!/usr/bin/env bash
set -euo pipefail

# Cross-compile Mirage for Windows from macOS/Linux.
# This produces the Windows binaries staged in package-resources.
# The MSI packaging step usually must run on Windows; this script will attempt it,
# but may fail if WiX/jpackage Windows tooling is unavailable.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DAEMON_DIR="${PROJECT_ROOT}/src/daemon_next"
KMP_DIR="${PROJECT_ROOT}/src/client-kmp"
RESOURCES_DIR="${KMP_DIR}/package-resources"

TARGET="x86_64-pc-windows-gnu"

cd "${PROJECT_ROOT}"
mkdir -p "${RESOURCES_DIR}/windows"

echo "[Windows cross] Ensuring Rust target ${TARGET} is installed..."
rustup target add "${TARGET}" 2>/dev/null || true

echo "[Windows cross] Building Rust daemon and CLI..."
cd "${DAEMON_DIR}"
cargo build --release --target "${TARGET}" --bin mirage-daemon --bin mirage

echo "[Windows cross] Staging binaries for Compose packaging..."
cp "${DAEMON_DIR}/target/${TARGET}/release/mirage-daemon.exe" "${RESOURCES_DIR}/windows/mirage-daemon.exe"
cp "${DAEMON_DIR}/target/${TARGET}/release/mirage.exe" "${RESOURCES_DIR}/windows/mirage.exe"

echo "[Windows cross] Attempting MSI packaging via Gradle..."
cd "${KMP_DIR}"
if ./gradlew packageMsi; then
    MSI_FILE=$(find "${KMP_DIR}/build/compose/binaries/main-release/msi" -maxdepth 1 -name "*.msi" -print -quit 2>/dev/null || true)
    if [[ -n "${MSI_FILE}" && -f "${MSI_FILE}" ]]; then
        echo "[Windows cross] MSI installer: ${MSI_FILE}"
    fi
else
    echo "[Windows cross] MSI packaging failed (expected on non-Windows hosts)."
    echo "[Windows cross] Windows binaries are ready in: ${RESOURCES_DIR}/windows/"
fi
