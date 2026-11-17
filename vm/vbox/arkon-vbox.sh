#!/usr/bin/env bash
# arkon-vbox.sh — Create and start an ARKON VirtualBox VM.
#
# Usage:  arkon-vbox.sh create [path/to/arkon.qcow2]
#         arkon-vbox.sh start
#         arkon-vbox.sh delete
#
# Requires: VirtualBox 7.x, VBoxManage in PATH.
# QCOW2 is converted to VMDK (VirtualBox does not natively support QCOW2).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VM_NAME="ARKON-OS"
VRAM=256
CPUS=4
RAM_MB=8192
QCOW2="${2:-${REPO_ROOT}/artifacts/qcow2/disk.qcow2}"
VMDK="${REPO_ROOT}/artifacts/qcow2/arkon.vmdk"
EFIVARS_DIR="${HOME}/VirtualBox VMs/${VM_NAME}"

# UEFI firmware: VirtualBox 7.x bundles its own EFI.

case "${1:-create}" in

create)
    if [[ ! -f "${QCOW2}" ]]; then
        echo "ERROR: QCOW2 not found: ${QCOW2}" >&2
        exit 1
    fi

    echo "==> Converting QCOW2 to VMDK for VirtualBox"
    VBoxManage convertfromraw --format VMDK "${QCOW2}" "${VMDK}" 2>/dev/null || \
    qemu-img convert -O vmdk "${QCOW2}" "${VMDK}"
    echo "    VMDK: ${VMDK}"

    echo "==> Creating VirtualBox VM: ${VM_NAME}"
    VBoxManage createvm --name "${VM_NAME}" --ostype Fedora_64 --register

    VBoxManage modifyvm "${VM_NAME}" \
        --cpus "${CPUS}" \
        --memory "${RAM_MB}" \
        --vram "${VRAM}" \
        --firmware efi \
        --graphicscontroller vmsvga \
        --accelerate3d off \
        --nic1 nat \
        --nictype1 virtio \
        --audio-driver none \
        --rtcuseutc on \
        --boot1 disk

    VBoxManage storagectl "${VM_NAME}" --name "SATA" --add sata --controller IntelAhci
    VBoxManage storageattach "${VM_NAME}" \
        --storagectl "SATA" --port 0 --device 0 \
        --type hdd --medium "${VMDK}"

    echo "==> VM created. Start it with: $0 start"
    ;;

start)
    VBoxManage startvm "${VM_NAME}" --type gui
    ;;

delete)
    VBoxManage controlvm "${VM_NAME}" poweroff 2>/dev/null || true
    VBoxManage unregistervm "${VM_NAME}" --delete
    echo "==> VM deleted."
    ;;

*)
    echo "Usage: $0 [create|start|delete] [qcow2-path]" >&2
    exit 1
    ;;

esac
