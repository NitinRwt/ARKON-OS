# ARKON OS — Architecture Index

This file is the map. The detail lives in [docs/](docs/); do not duplicate it here.

## One-paragraph summary

ARKON OS is a Fedora bootc image (Fedora 45 today; it tracks the current release): the whole operating system is built in CI as a signed container image, installed to physical hardware with Anaconda, and updated atomically with `bootc upgrade`, keeping the previous version bootable. Image variants are chosen by GPU: `arkon` (Mesa) and `arkon-nvidia` (NVIDIA kernel modules, prebuilt and signed; open flavour by default). The desktop is KDE Plasma 6.8 on Wayland. Games run through a native Steam client and Proton. Development and AI/ML work happens in Podman containers, with the GPU passed in through CDI. A small set of Rust components and one Tauri application (ARKON Center) expose update, hardware and diagnostics functions without giving applications root.

## Layer map

```
┌──────────────────────────────────────────────────────────────────────┐
│ User space (mutable)   Flatpak apps · Podman dev/AI containers · /home │
├──────────────────────────────────────────────────────────────────────┤
│ ARKON layer            ARKON Center (Tauri) · arkonctl · session agent │
│                        arkon-updated · arkon-secureboot · arkon-diag   │
│                        arkon-hw library + arkon-hwprobe CLI   (Rust)   │
├──────────────────────────────────────────────────────────────────────┤
│ Gaming layer           Steam (RPM) · Gamescope · MangoHud              │
├──────────────────────────────────────────────────────────────────────┤
│ Desktop layer          KDE Plasma 6.8 · Wayland · PipeWire             │
├──────────────────────────────────────────────────────────────────────┤
│ Graphics layer         Mesa (all)  |  NVIDIA driver + signed kmods     │
├──────────────────────────────────────────────────────────────────────┤
│ Base image (read-only) Fedora bootc · Fedora kernel · systemd          │
├──────────────────────────────────────────────────────────────────────┤
│ Boot chain             UEFI Secure Boot → shim → GRUB2 → kernel        │
└──────────────────────────────────────────────────────────────────────┘
```

## Where to read

| Topic | Document |
|---|---|
| Goals, users, scope, non-goals | [docs/PRODUCT_VISION.md](docs/PRODUCT_VISION.md) |
| Components, data flow, build pipeline, service interfaces | [docs/SYSTEM_ARCHITECTURE.md](docs/SYSTEM_ARCHITECTURE.md) |
| Supported hardware, tiers, GPU-to-variant mapping | [docs/HARDWARE_SUPPORT_MATRIX.md](docs/HARDWARE_SUPPORT_MATRIX.md) |
| Trust chain, signing, privilege model, threat model | [docs/SECURITY_ARCHITECTURE.md](docs/SECURITY_ARCHITECTURE.md) |
| Update streams, rollback, recovery paths | [docs/UPDATE_AND_RECOVERY_DESIGN.md](docs/UPDATE_AND_RECOVERY_DESIGN.md) |
| Steam, Proton, Gamescope, measurement | [docs/GAMING_ARCHITECTURE.md](docs/GAMING_ARCHITECTURE.md) |
| Containers, CDI, CUDA, PyTorch, TensorFlow | [docs/AI_DEVELOPER_ARCHITECTURE.md](docs/AI_DEVELOPER_ARCHITECTURE.md) |
| Directory layout, ownership, dependency rules | [docs/REPOSITORY_STRUCTURE.md](docs/REPOSITORY_STRUCTURE.md) |
| Phases, order, estimates | [docs/DEVELOPMENT_ROADMAP.md](docs/DEVELOPMENT_ROADMAP.md) |
| Phase gates | [docs/ACCEPTANCE_CRITERIA.md](docs/ACCEPTANCE_CRITERIA.md) |
| Risks | [docs/RISK_REGISTER.md](docs/RISK_REGISTER.md) |
| Decisions and open questions | [DECISIONS.md](DECISIONS.md) |

## Dependency map

Build-time and run-time dependencies between ARKON-owned parts. An arrow means "depends on". Nothing may depend upward.

```
Build-time (crate dependencies)
  center (Tauri) ──► arkon-dbus-api, arkon-hw
  arkonctl ────────► arkon-dbus-api, arkon-hw
  arkon-updated ───► arkon-dbus-api, arkon-hw, arkon-common
  arkon-secureboot, arkon-diag, arkon-session-agent ──► arkon-dbus-api, arkon-common
  arkon-hwprobe ───► arkon-hw
  arkon-hw ────────► nothing ARKON-owned

Run-time
  center, arkonctl, session agent ──D-Bus──► arkon-updated, arkon-secureboot, arkon-diag
  arkon-updated ──starts──► arkon-bootc@ worker units ──► bootc
  arkon-secureboot ──► mokutil
  arkon-diag ──► journald; arkon-updated over D-Bus; arkon-hwprobe
  session agent ──► arkon-hwprobe; flatpak (user scope)
  installer environment ──► arkon-hwprobe
  health checks ──► arkon-hwprobe; arkon-updated over D-Bus

image/  ──► kmods/ (nvidia variants only) ──► kernel of the pinned base digest
image/  ──► packaging/ (arkon-* RPMs built from services/, center/, branding/)
installer/ ──► image/ (published, signed image references); arkon-hwprobe RPM
bench/, tests/ ──► a booted ARKON system; never imported by product code
```

External dependencies and their sources:

| Dependency | Source | Used by |
|---|---|---|
| Base OS, kernel, Mesa, KDE, Podman, Gamescope, MangoHud, greenboot, bootupd | Fedora repositories | image |
| NVIDIA driver and kernel module source | RPM Fusion nonfree (D-007) | kmods, image (nvidia variant) |
| Steam client | RPM Fusion nonfree (D-017) | image |
| nvidia-container-toolkit | Fedora repositories | image (nvidia variant) |
| Graphical applications and the Flatpak NVIDIA runtime | Flathub | user, session agent |
| CUDA, PyTorch, TensorFlow images | Upstream registries | user containers |
| image-builder | osbuild project | installer |
| cosign | Sigstore | CI, client policy |
