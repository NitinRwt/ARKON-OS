#!/usr/bin/env bash
# convert-and-setup.sh — Convert the ARKON QCOW2 to VMDK for VMware.
#
# Run this on a Linux host after building the QCOW2.
# The resulting arkon.vmdk is placed alongside arkon.vmx for direct import.
#
# Usage: ./convert-and-setup.sh [path/to/arkon.qcow2]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
QCOW2="${1:-${REPO_ROOT}/artifacts/qcow2/disk.qcow2}"
VMDK="${SCRIPT_DIR}/arkon.vmdk"

if [[ ! -f "${QCOW2}" ]]; then
    echo "ERROR: QCOW2 not found: ${QCOW2}" >&2
    echo "       Build it first: cd ${REPO_ROOT} && just build-qcow2" >&2
    exit 1
fi

if ! command -v qemu-img &>/dev/null; then
    echo "ERROR: qemu-img not found. Install: dnf install qemu-img  (Fedora)" >&2
    exit 1
fi

echo "==> Converting QCOW2 to VMDK"
echo "    Source: ${QCOW2}"
echo "    Target: ${VMDK}"
qemu-img convert -p -O vmdk -o adapter_type=lsilogic "${QCOW2}" "${VMDK}"
echo "==> Done. Open vm/vmware/arkon.vmx in VMware Workstation or Fusion."
