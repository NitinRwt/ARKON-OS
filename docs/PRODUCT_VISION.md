# ARKON OS — Product Vision

## What ARKON OS is

A general-purpose Linux distribution for people who game, write software and run AI/ML workloads on the same physical machine. It installs directly on hardware, updates as a whole signed image, and can always boot the previous version.

It is a real distribution: its own images, installer, signing keys, update streams, release process and support policy. It is not a theme, a post-install script collection or a virtual appliance.

## Who it is for

| User | What they need from the OS |
|---|---|
| PC gamer moving from Windows | Steam works after install, the right GPU driver is already there, an update never leaves them at a black screen |
| Software engineer | A stable host, reproducible development environments that do not pollute it, containers that work rootless |
| AI/ML developer | NVIDIA GPU usable from containers on first boot, CUDA versions chosen per project instead of per machine |
| Everyday user | A desktop that updates itself safely and does not need a terminal |

## Product promises

Each promise has a matching acceptance criterion in [ACCEPTANCE_CRITERIA.md](ACCEPTANCE_CRITERIA.md).

1. **Installs on real hardware** from a USB stick, under UEFI with Secure Boot on.
2. **The right graphics driver without user effort.** The installer picks the image variant from the detected GPU.
3. **Updates cannot strand the machine.** An interrupted or bad update leaves a bootable previous version, selectable from the boot menu and chosen automatically after repeated boot failures.
4. **Games run through Steam and Proton** with no manual driver, library or kernel work. Not every Windows game works; ARKON says so.
5. **GPU containers work on first boot** on supported NVIDIA hardware.
6. **Nothing runs as root that does not need to**, and no application receives root.
7. **No telemetry.**
8. **Performance claims are published only with reproducible measurements** against stock Fedora KDE on the same machine.

## Scope of the first release (1.0)

In scope:

- Two image variants: Mesa (AMD/Intel GPUs) and NVIDIA (Turing and newer); possibly a third for Turing laptops, decided by measurement (U-14).
- Anaconda-based installer ISO, whole-disk installation, optional passphrase encryption.
- KDE Plasma 6.8 desktop with ARKON branding and sensible defaults.
- ARKON Center: update status and control, rollback, hardware and driver status, diagnostics export.
- Steam, Proton, Gamescope (as a launch wrapper), MangoHud.
- Podman, a container-based development workflow, NVIDIA GPU access in containers.
- Automatic rollback on failed boots, documented recovery paths.
- A published benchmark baseline against Fedora KDE 45.

## Non-goals for 1.0

| Not in 1.0 | Reason |
|---|---|
| Custom kernel, custom scheduler, x86-64-v3 rebuilds | No measured justification yet (D-021) |
| Console-style "boot to Steam" session | Not in Fedora; a large maintenance surface |
| Legacy NVIDIA GPUs (GTX 900/10 series and older) | Separate driver branch with a short remaining life (U-09) |
| AMD ROCm and Intel oneAPI GPU compute | No test hardware; untested support is not support |
| Automated dual-boot partitioning beside Windows | Destructive-risk surface; separate-disk installs only |
| Sealed, measured boot with systemd-boot and UKIs | Upstream gaps (D-003); tracked as the post-1.0 security target |
| TPM automatic disk unlock, hibernation | Not safe under the 1.0 boot chain, and blocked by kernel lockdown respectively (D-030) |
| A promise that rollback restores user data | Rollback restores the operating system; `/var` and home directories are not reverted |
| ARM, legacy BIOS, handheld form factors | Out of charter scope |
| An ARKON package repository or app store | Flatpak and containers cover the need |
| Automatic overclocking or undervolting | Charter restriction |
| AI assistants or cloud features inside the OS | Not a stated requirement; conflicts with the no-telemetry promise if done carelessly |

## How ARKON differs from what exists

Honest positioning matters because the closest existing project, Bazzite, already ships a Fedora-based, image-based gaming desktop with NVIDIA images.

| | Fedora KDE | Bazzite | ARKON OS (intended) |
|---|---|---|---|
| Update model | Package-based | Image-based | Image-based |
| NVIDIA driver | User installs | Preinstalled | Preinstalled, variant chosen by installer |
| Kernel | Fedora | Custom gaming kernel | Fedora, unmodified |
| Primary focus | General | Gaming, handhelds | Gaming, development and AI/ML with equal weight |
| System management UI | KDE tools | Terminal helper scripts and portal | Typed D-Bus services with one graphical front end |
| Performance claims | None | Community | Published, reproducible, against Fedora on the same hardware |

The differentiators ARKON has to earn are the last three rows. If Phase 8 shows no measurable advantage over Fedora, the product's value is integration and recovery, and the project must say that plainly.

## Success measures for 1.0

- All "must" criteria in ACCEPTANCE_CRITERIA.md pass on at least the reference laptop, and on one RTX desktop before the word "RTX" appears in any public claim (U-11).
- Zero known update paths that end in an unbootable system.
- A stranger can verify a released image's signature and list every package in it with its version and source. Builds are traceable, not bit-for-bit reproducible, because Fedora's repositories change between builds.
