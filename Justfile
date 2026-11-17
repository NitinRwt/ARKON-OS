# Justfile — ARKON OS build entry point.
# Install just: https://github.com/casey/just
# All build recipes require a Linux host (podman, image-builder).
# The 'test' recipe works on Windows (cargo).

VARIANT := "arkon"
QCOW2   := "artifacts/qcow2/disk.qcow2"
ISO     := "artifacts/iso/arkon.iso"

# Default: list available recipes.
default:
    @just --list

# ── Rust ───────────────────────────────────────────────────────────────────────

# Run the full unit-test suite (Windows-compatible, no hardware access).
test:
    cargo test --workspace

# Run tests with output for a specific crate.
test-crate crate:
    cargo test -p {{crate}} -- --nocapture

# Check all crates compile without warnings.
check:
    cargo check --workspace

# ── Container image ────────────────────────────────────────────────────────────

# Build the ARKON container image (requires Linux + podman).
build-image variant=VARIANT:
    bash build/scripts/build-image.sh {{variant}}

# ── QCOW2 disk image ───────────────────────────────────────────────────────────

# Build a bootable QCOW2 from the container image (requires Linux + podman + image-builder).
build-qcow2 variant=VARIANT:
    bash build/scripts/build-qcow2.sh {{variant}}

# ── Installer ISO ──────────────────────────────────────────────────────────────

# Build a generic installer ISO (requires Linux + podman + image-builder).
build-iso variant=VARIANT:
    bash build/scripts/build-iso.sh {{variant}}

# ── Verification ───────────────────────────────────────────────────────────────

# Verify checksums of all built artifacts.
verify:
    bash build/scripts/verify.sh

# ── VM launch ─────────────────────────────────────────────────────────────────

# Boot the QCOW2 in QEMU/KVM (requires Linux with /dev/kvm and OVMF).
qemu qcow2=QCOW2:
    bash vm/qemu/launch-arkon.sh {{qcow2}}

# ── Cleanup ───────────────────────────────────────────────────────────────────

# Remove built artifacts (keeps directory structure).
clean:
    bash build/scripts/clean.sh

# Remove built artifacts AND podman images.
clean-all:
    bash build/scripts/clean.sh --images

# ── All-in-one ────────────────────────────────────────────────────────────────

# Build image + QCOW2 + ISO for the given variant (Linux only).
all variant=VARIANT: (build-image variant) (build-qcow2 variant) (build-iso variant) verify
