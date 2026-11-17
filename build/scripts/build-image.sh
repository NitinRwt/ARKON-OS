#!/usr/bin/env bash
# build-image.sh — Build the ARKON bootc container image for one variant.
#
# Usage: build-image.sh [arkon|arkon-nvidia]
# Default: arkon
#
# Requirements:
#   - Linux host (Podman requires Linux; use this script inside WSL2 or CI)
#   - podman >= 4.x
#   - Internet access to quay.io/fedora/fedora-bootc:45 and RPM Fusion
#
# For arkon-nvidia: kmods/output/*.rpm must exist before running this script.
# See build/scripts/build-image.sh comment in image/Containerfile.
set -euo pipefail

VARIANT="${1:-arkon}"
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
IMAGE_TAG="localhost/arkon-${VARIANT}:latest"
CONTAINERFILE="${REPO_ROOT}/image/Containerfile"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_DIR="${REPO_ROOT}/artifacts/logs"
LOG_FILE="${LOG_DIR}/build-image-${VARIANT}-${TIMESTAMP}.log"

if [[ "${VARIANT}" != "arkon" && "${VARIANT}" != "arkon-nvidia" ]]; then
    echo "ERROR: unknown variant '${VARIANT}'. Use 'arkon' or 'arkon-nvidia'." >&2
    exit 1
fi

if [[ "${VARIANT}" == "arkon-nvidia" ]]; then
    if ! ls "${REPO_ROOT}/kmods/output/"*.rpm &>/dev/null; then
        echo "ERROR: kmods/output/*.rpm not found." >&2
        echo "       Build NVIDIA kernel modules first: just build-kmods open" >&2
        echo "       (or: just build-kmods closed  for the Turing closed-module flavour)" >&2
        exit 1
    fi
fi

mkdir -p "${LOG_DIR}"
echo "==> Building ${IMAGE_TAG}"
echo "    Log: ${LOG_FILE}"

# Record the base image digest before building so the build is reproducible.
BASE_IMAGE="quay.io/fedora/fedora-bootc:45"
echo "==> Pulling base image: ${BASE_IMAGE}"
podman pull "${BASE_IMAGE}" 2>&1 | tee -a "${LOG_FILE}"
BASE_DIGEST="$(podman inspect --format '{{.Digest}}' "${BASE_IMAGE}" 2>/dev/null || echo 'unknown')"
echo "    Base digest: ${BASE_DIGEST}" | tee -a "${LOG_FILE}"

# Build.
podman build \
    --target "${VARIANT}" \
    --tag "${IMAGE_TAG}" \
    --file "${CONTAINERFILE}" \
    --label "org.arkonos.build.timestamp=${TIMESTAMP}" \
    --label "org.arkonos.base.digest=${BASE_DIGEST}" \
    --no-cache \
    "${REPO_ROOT}" \
    2>&1 | tee -a "${LOG_FILE}"

echo "==> Build complete: ${IMAGE_TAG}"
echo "    Digest: $(podman inspect --format '{{.Digest}}' "${IMAGE_TAG}")"

# Record build metadata.
mkdir -p "${REPO_ROOT}/build/metadata"
METADATA_FILE="${REPO_ROOT}/build/metadata/build-${VARIANT}-${TIMESTAMP}.json"
cat > "${METADATA_FILE}" <<JSON
{
  "variant": "${VARIANT}",
  "image_tag": "${IMAGE_TAG}",
  "base_image": "${BASE_IMAGE}",
  "base_digest": "${BASE_DIGEST}",
  "build_timestamp": "${TIMESTAMP}",
  "log": "${LOG_FILE}"
}
JSON
echo "    Metadata: ${METADATA_FILE}"
