#!/usr/bin/env bash
# launch-arkon.sh — Boot ARKON OS in QEMU/KVM with UEFI firmware.
#
# Usage: launch-arkon.sh [path/to/arkon.qcow2]
# Default QCOW2: artifacts/qcow2/disk.qcow2
#
# Requirements:
#   - qemu-system-x86_64 with KVM support (/dev/kvm must exist)
#   - OVMF (UEFI firmware): usually in /usr/share/edk2/x64/ on Fedora/RHEL
#     or /usr/share/ovmf/ on Debian/Ubuntu
#   - 8+ GB free RAM
#
# This script does NOT touch any physical disk. The VM uses only the QCOW2
# image specified here.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
QCOW2="${1:-${REPO_ROOT}/artifacts/qcow2/disk.qcow2}"

if [[ ! -f "${QCOW2}" ]]; then
    echo "ERROR: QCOW2 not found: ${QCOW2}" >&2
    echo "       Build it first: just build-qcow2" >&2
    exit 1
fi

# UEFI firmware path — adjust for your distribution.
OVMF_CODE=""
for path in \
    /usr/share/edk2/x64/OVMF_CODE.4m.fd \
    /usr/share/edk2/x64/OVMF_CODE.fd \
    /usr/share/OVMF/OVMF_CODE_4M.fd \
    /usr/share/OVMF/OVMF_CODE.fd \
    /usr/share/ovmf/OVMF.fd; do
    if [[ -f "${path}" ]]; then
        OVMF_CODE="${path}"
        break
    fi
done

if [[ -z "${OVMF_CODE}" ]]; then
    echo "ERROR: OVMF UEFI firmware not found." >&2
    echo "       Install: dnf install edk2-ovmf  (Fedora/RHEL)" >&2
    echo "                apt install ovmf        (Debian/Ubuntu)" >&2
    exit 1
fi

# Create a fresh NVRAM file for this boot (EFI variables — reset each launch).
NVRAM_FILE="$(mktemp /tmp/arkon-nvram-XXXX.fd)"
trap "rm -f '${NVRAM_FILE}'" EXIT

# Locate the OVMF VARS template for the chosen CODE file.
OVMF_VARS="${OVMF_CODE/CODE/VARS}"
if [[ -f "${OVMF_VARS}" ]]; then
    cp "${OVMF_VARS}" "${NVRAM_FILE}"
fi

echo "==> Booting ARKON OS in QEMU"
echo "    Image:  ${QCOW2}"
echo "    UEFI:   ${OVMF_CODE}"
echo "    vCPUs:  4"
echo "    RAM:    8192 MB"
echo ""
echo "    Use Ctrl+Alt+G to release mouse, Ctrl+Alt+F to toggle full screen."
echo ""

exec qemu-system-x86_64 \
    -enable-kvm \
    -cpu host \
    -smp 4,sockets=1,cores=4,threads=1 \
    -m 8192 \
    -machine type=q35,acpi=on \
    \
    -drive if=pflash,format=raw,unit=0,file="${OVMF_CODE}",readonly=on \
    -drive if=pflash,format=raw,unit=1,file="${NVRAM_FILE}" \
    \
    -drive if=virtio,format=qcow2,file="${QCOW2}",cache=writeback,discard=unmap \
    \
    -netdev user,id=net0,hostfwd=tcp::2222-:22 \
    -device virtio-net-pci,netdev=net0 \
    \
    -device virtio-vga-gl \
    -display sdl,gl=on \
    \
    -device virtio-balloon-pci \
    -device virtio-rng-pci \
    \
    -rtc base=utc,clock=host \
    -boot menu=off \
    "$@"
