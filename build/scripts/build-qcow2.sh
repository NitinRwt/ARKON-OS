#!/usr/bin/env bash
# build-qcow2.sh — Generate a QCOW2 disk image from a built ARKON bootc image.
#
# Usage: build-qcow2.sh [arkon|arkon-nvidia]
# Default: arkon
set -euo pipefail

VARIANT="${1:-arkon}"
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
IMAGE_TAG="localhost/arkon-${VARIANT}:latest"
# bib writes to /output/qcow2/disk.qcow2 — mount the artifacts/ root as /output
# so the file lands at artifacts/qcow2/disk.qcow2.
ARTIFACTS_DIR="${REPO_ROOT}/artifacts"
OUTPUT_DIR="${ARTIFACTS_DIR}/qcow2"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_DIR="${ARTIFACTS_DIR}/logs"
LOG_FILE="${LOG_DIR}/build-qcow2-${VARIANT}-${TIMESTAMP}.log"
CONFIG_FILE="${REPO_ROOT}/build/config/image-builder-qcow2.toml"
NAMED_QCOW2="${OUTPUT_DIR}/arkon-${VARIANT}-${TIMESTAMP}.qcow2"

IMAGE_BUILDER="quay.io/centos-bootc/bootc-image-builder:latest"

mkdir -p "${OUTPUT_DIR}" "${LOG_DIR}" "${ARTIFACTS_DIR}/checksums"

if ! podman image exists "${IMAGE_TAG}"; then
    echo "ERROR: image '${IMAGE_TAG}' not found." >&2
    echo "       Build it first: ./build/scripts/build-image.sh ${VARIANT}" >&2
    exit 1
fi

echo "==> Generating QCOW2 from ${IMAGE_TAG}"
echo "    Output: ${OUTPUT_DIR}"
echo "    Log:    ${LOG_FILE}"

# Mount artifacts/ as /output — bib writes qcow2/disk.qcow2 inside it.
podman run \
    --rm \
    --privileged \
    --pull=newer \
    --security-opt label=type:unconfined_t \
    -v /var/lib/containers/storage:/var/lib/containers/storage \
    -v "${ARTIFACTS_DIR}":/output \
    -v "${CONFIG_FILE}":/config.toml:ro \
    "${IMAGE_BUILDER}" \
    --type qcow2 \
    --rootfs xfs \
    --config /config.toml \
    "${IMAGE_TAG}" \
    2>&1 | tee "${LOG_FILE}"

# bib writes to /output/qcow2/disk.qcow2.
BIB_QCOW2="${OUTPUT_DIR}/disk.qcow2"
if [[ ! -f "${BIB_QCOW2}" ]]; then
    # Fall back: search recursively in case bib layout changed.
    BIB_QCOW2="$(find "${OUTPUT_DIR}" -name "*.qcow2" | head -1)"
fi
if [[ -z "${BIB_QCOW2}" ]]; then
    echo "ERROR: QCOW2 not found under ${OUTPUT_DIR}" >&2
    exit 1
fi

# Rename to a timestamped name for the artifact upload.
mv "${BIB_QCOW2}" "${NAMED_QCOW2}"

echo "==> QCOW2 generated: ${NAMED_QCOW2}"
echo "    Size: $(du -sh "${NAMED_QCOW2}" | cut -f1)"

CHECKSUM_FILE="${ARTIFACTS_DIR}/checksums/arkon-${VARIANT}-${TIMESTAMP}.qcow2.sha256"
sha256sum "${NAMED_QCOW2}" > "${CHECKSUM_FILE}"
echo "    SHA256: $(cat "${CHECKSUM_FILE}")"
echo "    Checksum file: ${CHECKSUM_FILE}"
