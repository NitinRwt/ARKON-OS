#!/usr/bin/env bash
# clean.sh — Remove generated artifacts safely.
#
# Usage: clean.sh [artifacts|images|all]
# Default: artifacts
#
# This script never removes source files, scripts, or configuration.
# It only deletes files in artifacts/ and optionally removes the local
# Podman images (localhost/arkon-*).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TARGET="${1:-artifacts}"

clean_artifacts() {
    echo "==> Removing build artifacts"
    for dir in qcow2 iso checksums logs; do
        local path="${REPO_ROOT}/artifacts/${dir}"
        if [[ -d "${path}" ]]; then
            # Remove files but keep the directory itself.
            find "${path}" -type f ! -name '.gitkeep' -delete
            echo "    Cleaned: ${path}"
        fi
    done
    find "${REPO_ROOT}/build/metadata" -type f ! -name '.gitkeep' -delete
    echo "    Cleaned: ${REPO_ROOT}/build/metadata"
}

clean_images() {
    echo "==> Removing local Podman images"
    for img in arkon arkon-nvidia; do
        if podman image exists "localhost/${img}:latest"; then
            podman rmi "localhost/${img}:latest"
            echo "    Removed: localhost/${img}:latest"
        else
            echo "    Not found (skip): localhost/${img}:latest"
        fi
    done
}

case "${TARGET}" in
    artifacts) clean_artifacts ;;
    images)    clean_images ;;
    all)       clean_artifacts; clean_images ;;
    *)
        echo "ERROR: unknown target '${TARGET}'. Use artifacts, images, or all." >&2
        exit 1
        ;;
esac

echo "==> Done"
