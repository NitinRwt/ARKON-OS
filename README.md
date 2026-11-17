# ARKON OS

A Fedora-based operating system built as a signed container image, designed for gaming, AI/ML development, and daily use. Atomic updates, Secure Boot, KDE Plasma 6 on Wayland.

---

## What it is

ARKON OS is a [bootc](https://containers.github.io/bootc/) image built on Fedora 45. The entire OS ships as a signed container image, installed to hardware via an Anaconda ISO, and updated atomically — the previous version stays bootable at all times.

```
┌────────────────────────────────────────────────────────────┐
│ User space      Flatpak apps · Podman dev/AI containers    │
├────────────────────────────────────────────────────────────┤
│ ARKON layer     ARKON Center · arkonctl · arkon-hw (Rust)  │
├────────────────────────────────────────────────────────────┤
│ Gaming layer    Steam · Gamescope · MangoHud · Proton       │
├────────────────────────────────────────────────────────────┤
│ Desktop layer   KDE Plasma 6.8 · Wayland · PipeWire        │
├────────────────────────────────────────────────────────────┤
│ Base image      Fedora bootc · Kernel · systemd            │
├────────────────────────────────────────────────────────────┤
│ Boot chain      UEFI Secure Boot → shim → GRUB2 → kernel   │
└────────────────────────────────────────────────────────────┘
```

## Variants

| Variant | GPU | Notes |
|---|---|---|
| `arkon` | AMD / Intel (Mesa) | Default |
| `arkon-nvidia` | NVIDIA (prebuilt signed kmods) | Requires kmods pipeline |

## Project status

| Item | State |
|---|---|
| Phase 0 — Architecture | Complete |
| Phase 1 — OS image + CI | In progress |
| Phase 2 — Installer | Rust crates implemented, not yet integrated |
| First bootable image | Not yet built |

See [docs/PROJECT_STATE.md](docs/PROJECT_STATE.md) for the full state.

## Repository layout

```
crates/          Rust components (arkon-hw, arkon-storage, arkon-boot, arkon-installer, …)
image/           Containerfile + package manifests + kargs
build/scripts/   Build automation (build-image.sh, build-qcow2.sh, build-iso.sh)
vm/              VM launch configs (QEMU, VirtualBox, VMware)
docs/            Architecture, design decisions, phase docs
artifacts/       Build output (gitignored — produced by CI)
```

## Building

Builds require a Linux host (podman + image-builder). On Windows use **GitHub Actions** or WSL2.

```bash
# Build the container image
just build-image

# Build a bootable QCOW2 (for QEMU testing)
just build-qcow2

# Build an installer ISO
just build-iso

# Run tests (works on Windows too)
just test
```

## Running in a VM (after building)

```bash
# QEMU/KVM — Linux host
just qemu

# Windows host — copy the QCOW2 here first, then:
vm\qemu\launch-arkon.bat
```

## CI

GitHub Actions workflows in [.github/workflows/](.github/workflows/):

- **Build ARKON ISO** — triggers on every push to `main`, produces a downloadable installer ISO
- **Build ARKON QCOW2** — manual trigger, produces a bootable disk image for VM testing

## Tech stack

- **Base:** Fedora 45, `fedora-bootc:45`
- **Desktop:** KDE Plasma 6.8, Wayland, SDDM
- **Gaming:** Steam (RPM Fusion), Gamescope, MangoHud, GameMode, Proton
- **Updates:** `bootc upgrade`, A/B atomic, greenboot health checks
- **Installer:** image-builder `generic-iso`, Anaconda
- **Rust crates:** hardware discovery, storage planner, boot planner, installer state machine, first-boot setup
- **Build:** podman, osbuild image-builder, GitHub Actions
