#!/usr/bin/env bash
# build-qcow2.sh — Generate a QCOW2 disk image from a built ARKON bootc image.
#
# Usage: build-qcow2.sh [arkon|arkon-nvidia]
# Default: arkon
#
# The output QCOW2 can be booted directly in QEMU (see vm/qemu/).
#
# Requirements:
#   - Linux host with Podman
#   - The ARKON bootc image must already be built (run build-image.sh first)
#   - Privileged Podman (--privileged) for the image builder
#   - /var/lib/containers/storage must be accessible
#
# image-builder note:
#   bootc-image-builder was archived 2026-06-18 and merged into image-builder
#   (osbuild project). The container reference below may need updating if the
#   osbuild project has moved it. Verify at: https://github.com/osbuild/image-builder
#
#   Known working container as of architecture review (2026-11-16):
#     quay.io/osbuild/image-builder
#   (UPDATE THIS if the archive results in a different container name/tag)
set -euo pipefail

VARIANT="${1:-arkon}"
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
IMAGE_TAG="localhost/arkon-${VARIANT}:latest"
OUTPUT_DIR="${REPO_ROOT}/artifacts/qcow2"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_DIR="${REPO_ROOT}/artifacts/logs"
LOG_FILE="${LOG_DIR}/build-qcow2-${VARIANT}-${TIMESTAMP}.log"
CONFIG_FILE="${REPO_ROOT}/build/config/image-builder-qcow2.toml"

IMAGE_BUILDER="quay.io/centos-bootc/bootc-image-builder:latest"

mkdir -p "${OUTPUT_DIR}" "${LOG_DIR}"

# Verify the bootc image exists.
if ! podman image exists "${IMAGE_TAG}"; then
    echo "ERROR: image '${IMAGE_TAG}' not found." >&2
    echo "       Build it first: ./build/scripts/build-image.sh ${VARIANT}" >&2
    exit 1
fi

echo "==> Generating QCOW2 from ${IMAGE_TAG}"
echo "    Output: ${OUTPUT_DIR}"
echo "    Log:    ${LOG_FILE}"

podman run \
    --rm \
    --privileged \
    --pull=newer \
    --security-opt label=type:unconfined_t \
    -v /var/lib/containers/storage:/var/lib/containers/storage \
    -v "${OUTPUT_DIR}":/output \
    -v "${CONFIG_FILE}":/config.toml:ro \
    "${IMAGE_BUILDER}" \
    --type qcow2 \
    --rootfs xfs \
    --config /config.toml \
    "${IMAGE_TAG}" \
    2>&1 | tee "${LOG_FILE}"

# Locate the generated file.
QCOW2_FILE="$(find "${OUTPUT_DIR}" -name "*.qcow2" | head -1)"
if [[ -z "${QCOW2_FILE}" ]]; then
    echo "WARNING: could not locate QCOW2 output; check ${LOG_FILE}" >&2
    exit 1
fi

echo "==> QCOW2 generated: ${QCOW2_FILE}"
echo "    Size: $(du -sh "${QCOW2_FILE}" | cut -f1)"

# Calculate SHA256 checksum.
CHECKSUM_FILE="${REPO_ROOT}/artifacts/checksums/arkon-${VARIANT}-${TIMESTAMP}.qcow2.sha256"
sha256sum "${QCOW2_FILE}" > "${CHECKSUM_FILE}"
echo "    SHA256: $(cat "${CHECKSUM_FILE}")"
echo "    Checksum file: ${CHECKSUM_FILE}"
