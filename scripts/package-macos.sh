#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DAEMON_DIR="${PROJECT_ROOT}/src/daemon_next"
KMP_DIR="${PROJECT_ROOT}/src/client-kmp"
RESOURCES_DIR="${KMP_DIR}/package-resources"

cd "${PROJECT_ROOT}"
mkdir -p "${RESOURCES_DIR}/macos"

echo "[macOS] Building Rust daemon and CLI..."
cd "${DAEMON_DIR}"
cargo build --release --bin mirage-daemon --bin mirage

echo "[macOS] Staging binaries for Compose packaging..."
cp "${DAEMON_DIR}/target/release/mirage-daemon" "${RESOURCES_DIR}/macos/mirage-daemon"
cp "${DAEMON_DIR}/target/release/mirage" "${RESOURCES_DIR}/macos/mirage"
chmod +x "${RESOURCES_DIR}/macos/mirage-daemon" "${RESOURCES_DIR}/macos/mirage"

echo "[macOS] Packaging DMG via Gradle..."
cd "${KMP_DIR}"
./gradlew packageDmg

APP_BUNDLE="${KMP_DIR}/build/compose/binaries/main-release/app/Mirage.app"
DMG_DIR="${KMP_DIR}/build/compose/binaries/main-release/dmg"
DMG_FILE=$(find "${DMG_DIR}" -maxdepth 1 -name "*.dmg" -print -quit 2>/dev/null || true)

if [[ ! -d "${APP_BUNDLE}" ]]; then
    echo "[macOS] App bundle not found: ${APP_BUNDLE}"
    exit 0
fi

# Also copy the CLI and daemon into Contents/MacOS so the CLI can locate the daemon as a sibling.
MACOS_DIR="${APP_BUNDLE}/Contents/MacOS"
mkdir -p "${MACOS_DIR}"
cp "${RESOURCES_DIR}/macos/mirage-daemon" "${MACOS_DIR}/mirage-daemon"
cp "${RESOURCES_DIR}/macos/mirage" "${MACOS_DIR}/mirage"
chmod +x "${MACOS_DIR}/mirage-daemon" "${MACOS_DIR}/mirage"

echo "[macOS] Packaged app bundle: ${APP_BUNDLE}"
if [[ -n "${DMG_FILE}" && -f "${DMG_FILE}" ]]; then
    echo "[macOS] DMG installer: ${DMG_FILE}"
fi

echo "[macOS] To install: open the DMG and drag Mirage.app into /Applications."
