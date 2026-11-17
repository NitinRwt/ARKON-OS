#!/usr/bin/env bash
# verify.sh — Verify built artifacts: existence, file size, and SHA256 checksums.
#
# Usage: verify.sh [qcow2|iso|all]
# Default: all
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
QCOW2_DIR="${REPO_ROOT}/artifacts/qcow2"
ISO_DIR="${REPO_ROOT}/artifacts/iso"
CHECKSUM_DIR="${REPO_ROOT}/artifacts/checksums"

TARGET="${1:-all}"
PASS=0
FAIL=0

check_artifact() {
    local path="$1"
    local min_size_bytes="${2:-0}"
    local desc="$3"

    if [[ ! -f "${path}" ]]; then
        echo "FAIL  ${desc}: file not found (${path})"
        FAIL=$((FAIL + 1))
        return
    fi

    local size
    size=$(stat -c%s "${path}")
    if [[ "${size}" -lt "${min_size_bytes}" ]]; then
        echo "FAIL  ${desc}: too small (${size} bytes < ${min_size_bytes})"
        FAIL=$((FAIL + 1))
        return
    fi

    echo "PASS  ${desc}: $(du -sh "${path}" | cut -f1) (${path})"
    PASS=$((PASS + 1))
}

verify_checksums() {
    local dir="$1"
    local ext="$2"

    for ckfile in "${CHECKSUM_DIR}"/*."${ext}".sha256; do
        [[ -e "${ckfile}" ]] || continue
        artifact="${dir}/$(basename "${ckfile}" ".sha256")"
        if [[ ! -f "${artifact}" ]]; then
            echo "FAIL  checksum target not found: ${artifact}"
            FAIL=$((FAIL + 1))
            continue
        fi
        if sha256sum --check "${ckfile}" --quiet 2>/dev/null; then
            echo "PASS  checksum ok: $(basename "${artifact}")"
            PASS=$((PASS + 1))
        else
            echo "FAIL  checksum mismatch: $(basename "${artifact}")"
            FAIL=$((FAIL + 1))
        fi
    done
}

echo "==> Verifying ARKON build artifacts"
echo "    Artifact dirs: ${QCOW2_DIR}, ${ISO_DIR}"
echo ""

if [[ "${TARGET}" == "qcow2" || "${TARGET}" == "all" ]]; then
    echo "--- QCOW2 images ---"
    for f in "${QCOW2_DIR}"/*.qcow2; do
        [[ -e "${f}" ]] || { echo "    (none found)"; break; }
        # Minimum 500 MB for a usable QCOW2 image.
        check_artifact "${f}" $((500 * 1024 * 1024)) "QCOW2: $(basename "${f}")"
    done
    verify_checksums "${QCOW2_DIR}" "qcow2"
fi

if [[ "${TARGET}" == "iso" || "${TARGET}" == "all" ]]; then
    echo ""
    echo "--- ISO images ---"
    for f in "${ISO_DIR}"/*.iso; do
        [[ -e "${f}" ]] || { echo "    (none found)"; break; }
        # Minimum 1 GB for a bootable ISO.
        check_artifact "${f}" $((1 * 1024 * 1024 * 1024)) "ISO: $(basename "${f}")"
    done
    verify_checksums "${ISO_DIR}" "iso"
fi

echo ""
echo "==> Result: ${PASS} passed, ${FAIL} failed"
[[ "${FAIL}" -eq 0 ]]
