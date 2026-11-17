#!/usr/bin/env bash
# build-iso.sh — Generate a bootable installer ISO from a built ARKON bootc image.
#
# Usage: build-iso.sh [arkon|arkon-nvidia]
# Default: arkon
#
# ISO type: generic-iso (the current supported bootc installer type as of the
# architecture review on 2026-11-16, after bootc-image-builder was archived).
#
# The generic-iso creates an Anaconda-based installer ISO that:
#   1. Boots from USB/DVD.
#   2. Runs Anaconda with a kickstart or interactive UI.
#   3. Installs the bootc image to the target disk.
#
# Phase 2 installer status:
#   The arkon-hw hardware probe and arkon-storage partition planner are
#   implemented but arkon-installd (the D-Bus orchestrator) does not yet exist.
#   The ISO produced by this script will contain the base Anaconda installer
#   plus the ARKON bootc image, but the ARKON custom installer UI will not be
#   present until Phase 2 is complete.
#
# Requirements: same as build-qcow2.sh
set -euo pipefail

VARIANT="${1:-arkon}"
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
IMAGE_TAG="localhost/arkon-${VARIANT}:latest"
OUTPUT_DIR="${REPO_ROOT}/artifacts/iso"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_DIR="${REPO_ROOT}/artifacts/logs"
LOG_FILE="${LOG_DIR}/build-iso-${VARIANT}-${TIMESTAMP}.log"
CONFIG_FILE="${REPO_ROOT}/build/config/image-builder-iso.toml"

IMAGE_BUILDER="quay.io/osbuild/image-builder"

mkdir -p "${OUTPUT_DIR}" "${LOG_DIR}"

if ! podman image exists "${IMAGE_TAG}"; then
    echo "ERROR: image '${IMAGE_TAG}' not found." >&2
    echo "       Build it first: ./build/scripts/build-image.sh ${VARIANT}" >&2
    exit 1
fi

echo "==> Generating installer ISO from ${IMAGE_TAG}"
echo "    Type: generic-iso"
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
    build \
    --type generic-iso \
    --output /output \
    --config /config.toml \
    "${IMAGE_TAG}" \
    2>&1 | tee "${LOG_FILE}"

ISO_FILE="$(find "${OUTPUT_DIR}" -name "*.iso" -newer "${LOG_FILE}" | head -1)"
if [[ -z "${ISO_FILE}" ]]; then
    echo "WARNING: could not locate ISO output; check ${LOG_FILE}" >&2
    exit 1
fi

echo "==> ISO generated: ${ISO_FILE}"
echo "    Size: $(du -sh "${ISO_FILE}" | cut -f1)"

CHECKSUM_FILE="${REPO_ROOT}/artifacts/checksums/arkon-${VARIANT}-${TIMESTAMP}.iso.sha256"
sha256sum "${ISO_FILE}" > "${CHECKSUM_FILE}"
echo "    SHA256: $(cat "${CHECKSUM_FILE}")"
echo "    Checksum file: ${CHECKSUM_FILE}"
