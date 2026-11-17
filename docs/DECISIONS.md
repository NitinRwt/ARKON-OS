# ARKON OS — Decision Log

Status values: **Accepted** (fixed by the charter or approved by the owner), **Proposed** (recommended in Phase 0, awaiting owner approval), **Superseded**.
Evidence dates refer to research done on 2026-11-16. New decisions are appended; existing entries are edited only to change status.

## Decisions

### D-001 — Base: Fedora 45 through the official bootc base image
Status: Proposed (amended by D-031: the Fedora version tracks the current release)
Build from `quay.io/fedora/fedora-bootc:45` and install ARKON's own KDE Plasma package set, instead of deriving from the Fedora Kinoite container images or from Universal Blue.
Why: Fedora 45 was released 2026-11-03. Fedora's own Atomic Desktop bootable container images are still described by the Atomic SIG as unofficial, and deriving from Universal Blue would make ARKON a downstream of a downstream. The plain bootc base is the supported derivation point.
Cost: ARKON owns the desktop package manifest.
Verify: V-01.

### D-002 — x86_64, UEFI only
Status: Accepted (charter)
No legacy BIOS, no 32-bit host, no ARM in 1.0.

### D-003 — Boot chain for Phases 1–2: ostree backend, bootupd, GRUB2, Fedora shim
Status: Proposed — **deviates from the charter's systemd-boot preference**
Why not systemd-boot now: bootc supports systemd-boot only with its composefs backend. That backend is documented as stable for core functionality, but its documented gaps hit ARKON's requirements directly:
- no boot counting and no `bootc-boot-complete.service`, so automatic rollback after a failed boot (charter principle 3) has no upstream mechanism;
- kernel arguments are silently ignored on the BLS install path (bootc issue #2539), and the NVIDIA variant needs kernel arguments;
- there is no in-place transition from the ostree backend, so a later switch means reinstalling;
- a Secure Boot chain for systemd-boot needs ARKON-owned keys, because Fedora's published sealed images are test builds not signed with Fedora's keys.

Plan: ship Phases 1–2 on the Fedora-default chain, which is Fedora-signed end to end. Run a time-boxed systemd-boot + composefs spike in Phase 1 (task P1-12). Re-decide before the Phase 2 gate, and again before Phase 9, because after public release a backend switch forces every user to reinstall (R-04).

### D-004 — Unmodified Fedora kernel, pinned per image build
Status: Accepted (charter)
The kernel version is fixed at image build time. Kernel modules are built for exactly that version. No custom kernel in 1.0.

### D-005 — Two image variants in 1.0
Status: Proposed (amended by D-032: a third, closed-module variant is possible pending U-14)
- `arkon` — Mesa only. AMD and Intel GPUs.
- `arkon-nvidia` — NVIDIA current driver branch with the open kernel modules. Turing and newer (GTX 16, RTX 20/30/40/50).

No legacy NVIDIA variant (580 branch for Maxwell/Pascal/Volta) in 1.0; see U-09.
Why: every variant multiplies build, signing and test cost, and the project has one test machine.

### D-006 — NVIDIA kernel modules are prebuilt and signed in CI
Status: Proposed
Modules are compiled in CI against the pinned kernel, signed with an ARKON module-signing key, and installed into the image as RPMs. The public certificate ships in the image; users enrol it as a Machine Owner Key once. No akmods builds on user machines.
Why: an image-based system cannot rebuild modules into `/usr` at runtime, and a kernel/module mismatch is the most common NVIDIA failure on Fedora.

### D-007 — Third-party sources are an explicit allow-list
Status: Proposed (depends on U-05)
Allowed: Fedora repositories, RPM Fusion free and nonfree, Flathub. Each entry has its GPG key fingerprint committed to the repository, package versions recorded in a lock file per build, and an SBOM per image. COPR and ad-hoc repositories are not allowed in release builds.
Why: the NVIDIA driver and Steam do not exist in Fedora's own repositories. "Verified" is defined here as signed, pinned and recorded.

### D-008 — Secure Boot stays enabled
Status: Accepted (charter)
ARKON reuses Fedora's Microsoft-signed shim and Fedora-signed GRUB and kernel. The Mesa variant boots under Secure Boot with no user action. The NVIDIA variant needs one MOK enrolment. Documentation never recommends disabling Secure Boot.

### D-009 — Signed images, enforced on the client
Status: Proposed (mechanism corrected by D-026)
Images are signed with cosign in CI. The installed system rejects unsigned or wrongly signed ARKON images, so `bootc upgrade` cannot pull an untrusted update.

### D-010 — Updates: bootc A/B deployments, staged, with automatic rollback
Status: Proposed
`bootc upgrade` stages the new image; it becomes active at the next reboot the user chooses. The previous deployment stays bootable. Health checks trigger rollback after repeated failed boots. Streams: `stable`, `testing`, `dev`. Details in [docs/UPDATE_AND_RECOVERY_DESIGN.md](docs/UPDATE_AND_RECOVERY_DESIGN.md).

### D-011 — Btrfs root, optional LUKS2
Status: Proposed
Btrfs with zstd compression, matching Fedora's desktop default. Full-disk encryption is offered in the installer and is opt-in (U-12).

### D-012 — Installer is Anaconda, built with image-builder
Status: Proposed — would be superseded by D-034 if the owner approves it (U-19). image-builder remains the ISO tool either way
The ISO is produced by osbuild's `image-builder` with the `generic-iso` type. ARKON does not write its own installer.
Why: `bootc-image-builder` was archived on 2026-06-18 and merged into `image-builder`; the `anaconda-iso` type is listed as historical.

### D-013 — KDE Plasma 6.8, Wayland only
Status: Accepted (charter, and upstream reality)
Plasma 6.8 has no X11 session. X11 applications run through Xwayland.

### D-014 — Applications come from Flatpak and containers
Status: Proposed
Graphical applications: Flatpak. Development environments: Podman containers. Client-side RPM layering is not a supported workflow.

### D-015 — System services in Rust over D-Bus with polkit
Status: Proposed (component list corrected by D-024 and D-025)
Small, single-purpose services on the system bus, each with the least privilege it needs, authorised per method through polkit. Interface definitions are the typed contract. See [docs/SYSTEM_ARCHITECTURE.md](docs/SYSTEM_ARCHITECTURE.md).

### D-016 — ARKON Center: Tauri 2 + React + TypeScript, shipped as an RPM in the image
Status: Accepted (charter), with a spike gate
Risk: Tauri on Linux renders with WebKitGTK, which has a history of rendering faults on NVIDIA + Wayland and pulls a GTK stack into a KDE system. Phase 4 starts with a rendering spike on the reference laptop (acceptance criterion 4.1). If it fails, the fallback is a Qt/Kirigami front end over the same D-Bus API.

### D-017 — Steam as a native RPM; Proton brings its own DXVK and VKD3D-Proton
Status: Proposed (depends on U-06)
ARKON does not package DXVK or VKD3D-Proton separately; they ship inside Proton, which Steam manages.
Cost: about 195 i686 packages enter the image.

### D-018 — AI/ML stacks live in containers
Status: Proposed
The host carries the NVIDIA driver and `nvidia-container-toolkit` only. CUDA, PyTorch and TensorFlow come from upstream container images, with the GPU passed in through CDI.

### D-019 — No telemetry
Status: Accepted (charter)
Nothing leaves the machine without an explicit user action. Diagnostics bundles are created locally on request.

### D-020 — GitHub Actions is the build system of record; bare metal is the acceptance target
Status: Proposed (depends on U-02, U-03)
All release artefacts are built in CI from a clean checkout. Acceptance tests run on physical hardware.

### D-021 — No speculative optimisation in 1.0
Status: Accepted (charter)
No custom scheduler, no x86-64-v3 package rebuild, no kernel patches and no overclocking until Phase 8 has a measured baseline that justifies a specific change.

### D-022 — ARKON branding replaces Fedora branding
Status: Proposed
`arkon-release` and `arkon-logos` replace `fedora-release` and `fedora-logos`. `os-release` carries `ID=arkon`, `ID_LIKE=fedora`.
Why: Fedora's trademark guidelines do not let a derived distribution present itself with Fedora's marks.

## Decisions added by the Phase 0 architecture review (2026-11-16)

### D-023 — The base image digest is the kernel pin; repositories are recorded, not pinned
Status: Proposed
The kernel is whatever the pinned `fedora-bootc` digest contains, and no build step changes kernel packages. Packages added on top come from the repositories as they are at build time and are recorded in the lock file. If NVIDIA modules do not build for a new base, the Mesa variant advances and the NVIDIA variant stays on the last good base with userspace-only updates, for a bounded time fixed in Phase 1.
Why: the earlier text claimed "pinned inputs" and a shared base while also promising independent variant schedules. Those cannot all hold.

### D-024 — Hardware detection is one library with a CLI, not a daemon
Status: Proposed — replaces `arkon-hwd` in D-015
`arkon-hw` (library) and `arkon-hwprobe` (CLI) are the only implementation of hardware detection and of the GPU-to-variant rule. The installer, the updater, the session agent, `arkonctl` and ARKON Center all use them.
Why: the installer needs the logic where no service runs, the data is readable without privilege, and two implementations of the variant rule would drift.

### D-025 — Service boundary corrections
Status: Proposed — amends D-015
- `arkon-updated` is split: an unprivileged, sandboxed D-Bus front, and fixed root worker units that each run one predefined `bootc` operation.
- Selectable streams and variants come from a read-only list inside the signed image, not from a list compiled into the service.
- Diagnostics have two levels. The basic level is open to the active user; the level that includes system journal content needs administrator authentication. Bundles are returned as a file descriptor.
- A user-session agent delivers notifications and keeps the Flatpak NVIDIA runtime in step.
- `QueueEnrollment` generates the one-time password itself.

### D-026 — Signature policy is scoped to the ARKON registry
Status: Proposed — corrects D-009
The system-wide container policy keeps Fedora's default. A scoped rule requires a valid ARKON signature for the ARKON registry path, and bootc's signature enforcement is enabled explicitly at install and on every switch.
Why: a default-reject policy would block every upstream container pull, which the development and AI workflows depend on.
Verify: V-14.

### D-027 — The updater refuses images older than the booted one
Status: Proposed
Compared by the build timestamp label. Explicit rollback is unaffected.
Why: a signature does not prevent replay of an old signed image.

### D-028 — The Flatpak NVIDIA runtime is kept in step with the host driver
Status: Proposed
When a staged image changes the driver version, the session agent fetches the matching Flatpak runtime before the reboot and keeps the current one. An NVIDIA image is promoted to `stable` only once Flathub has published the matching runtime.
Why: Flatpak is ARKON's application channel (D-014), and GPU-using Flatpaks break when the runtime and host driver versions differ.

### D-029 — The module-signing certificate is restricted to module signing
Status: Proposed
Issued with extended key usage OID 1.3.6.1.4.1.2312.16.1.2, so shim does not accept it for boot loaders or kernels.
Why: an unrestricted MOK certificate would turn a module-key leak into a full Secure Boot bypass.
Verify: V-12.

### D-030 — No TPM automatic unlock and no hibernation in 1.0
Status: Proposed
Disk encryption uses a passphrase. Hibernation is unsupported.
Why: under the GRUB chain, a TPM binding that survives updates does not cover the kernel command line, which can be edited at the boot menu. Kernel lockdown disables hibernation under Secure Boot.

### D-031 — Development tracks the current Fedora release
Status: Proposed — amends D-001
The base is Fedora 45 today. ARKON rebases when a new Fedora release has passed the bare-metal suite, and 1.0 ships on the release that is current at beta with at least six months of upstream support left.
Why: the roadmap is longer than the remaining life of Fedora 45.

### D-032 — The NVIDIA module flavour is a build parameter
Status: Proposed — amends D-005
The kmods pipeline builds open or closed modules on request. Open is the default and is required for RTX 50. A closed-module variant for Turing is added only if U-14 decides so.
Why: NVIDIA documents runtime power-down for the open modules on Ampere and newer only. On a Turing hybrid laptop such as R1 the discrete GPU may never power down under the open modules.

### D-033 — Keep Fedora's automatic bootloader updates; gate them on a bare-metal boot
Status: Proposed
`bootloader-update.service` stays enabled. An image that changes shim or GRUB is promoted to `stable` only after booting on R1 with Secure Boot on. ARKON does not push firmware revocation updates.
Why: bootloader updates are outside the A/B mechanism and cannot be rolled back, yet a stale boot chain can stop booting after a firmware revocation update.
Verify: V-15.

## Decisions proposed by the Phase 2 design draft (2026-11-16)

Drafted before Phase 1 exists, at the owner's instruction. All are proposals and all depend on verification items V-17…V-31. Detail is in [docs/phase2/](docs/phase2/).

### D-034 — Installer engine: systemd-repart and `bootc install to-filesystem` under an ARKON orchestrator
Status: Proposed — replaces D-012; needs U-19
Partitioning, formatting and encryption by systemd-repart from declarative definitions; deployment and bootloader by bootc; an ARKON orchestrator and front end around them. Anaconda driven by a generated kickstart is kept as a fallback for whole-disk installs only.
Why: Anaconda's bootc path is a preview limited to single-disk layouts without custom mount points, Fedora documents dual boot and manual partitioning on its image-based desktops as not fully functional, and bootc's own documentation designates `to-filesystem` for distribution installers.
Cost: the largest scope increase since Phase 0; roughly 14–17 engineer-weeks for Phase 2 instead of 4–6.

### D-035 — ARKON never writes to an EFI system partition it did not create
Status: Proposed
Every installation gets an ARKON-owned ESP, including a second ESP for same-disk dual boot.
Why: bootc's installation empties the ESP it is given, and ARKON's boot files use Fedora's vendor directory. Sharing an ESP would destroy another system's boot files.

### D-036 — The installer never shrinks, moves or reformats an existing partition
Status: Proposed
The only way existing data is removed is erasing a whole disk the user chose. Installing beside another system requires unallocated space that already exists; the user frees it with that system's own tools.

### D-037 — User and locale settings reach the installed system through a first-boot handoff
Status: Proposed
The installer writes one root-only file to the new installation's persistent state; a first-boot service applies and deletes it. The installer does not edit files inside the deployment.

### D-038 — One partition layout: ESP 1 GiB, `/boot` 2 GiB ext4, Btrfs with `root` and `home` subvolumes, optional LUKS2
Status: Proposed — extends D-011; sizes provisional until Phase 1 measurements
The `home` subvolume exists so the OS can be replaced without touching user data.

### D-039 — Destructive execution needs a plan-bound confirmation enforced by the orchestrator
Status: Proposed
A single-use token is issued only for the matching plan hash, a typed phrase derived from the target disk, and a fresh scan identical to the one the plan was built on.

### D-040 — The plan shown is the partitioner's own dry run, and is validated before and after execution
Status: Proposed
In non-erase modes a plan is rejected unless every pre-existing partition is reported unchanged, and the disk is compared with the plan again after partitioning.

## Open decisions

### Blocking the Phase 0 gate

**U-01 — Bare-metal test target.**
The only machine has one disk, and Windows is on it. Options: (a) dedicated external SSD, 128 GB or larger, selected as the install target and booted from the firmware boot menu; (b) a second internal drive, if the chassis has a free slot — not checked; (c) a separate test machine.
Recommendation (revised in review): (b) if the laptop has a free internal slot, otherwise (a). An OS running from a USB disk makes boot-time and storage measurements unrepresentative and makes suspend/resume unreliable, so with (a) those tests are recorded as limited. The assistant will not run partitioning or installation commands; the owner performs installs.
Before any install or firmware change on this laptop, whichever option is chosen: check whether Windows device encryption (BitLocker) is active and save its recovery key, back up the Windows data, and note the current firmware boot order. Installing a second OS adds a boot entry to firmware, and firmware or Secure Boot changes can make Windows ask for the recovery key.

**U-02 — GitHub organisation, visibility, registry.**
Needed before any CI or image publishing. Recommendation: public repository, images on `ghcr.io/<org>/arkon`.

**U-03 — QEMU boot checks in CI.**
The charter rules out virtual machines as the primary development or deployment solution. A boot smoke test in QEMU on each pull request would catch unbootable images before anyone flashes hardware. Recommendation: allow it as a secondary check that can never satisfy a phase gate on its own.

**U-04 — Approve D-003** (GRUB2 + shim now, systemd-boot re-evaluated).

**U-05 — Approve D-007** (RPM Fusion and Flathub allow-listed). Without RPM Fusion or an equivalent, there is no NVIDIA driver and no native Steam.

**U-06 — Steam delivery.**
Native RPM: best compatibility, required for any future console-style session, adds i686 packages to the image. Flatpak: smaller image and sandboxed, but its NVIDIA runtime must match the host driver version exactly and lags after driver updates. Recommendation: native RPM.

**U-07 — Project identity.**
(a) A reverse-DNS prefix the project controls, for D-Bus names, polkit actions and app IDs. The documents use `org.arkonos` as a placeholder. (b) Source licence. (c) A trademark search for "ARKON" — not done; other organisations use the name.

### Not blocking Phase 1

**U-08 — Signing key custody.** GitHub Actions secrets are the simple option; a cloud KMS or hardware token is safer for the module-signing key. Needed before Phase 1 publishes an NVIDIA image.

**U-09 — Legacy NVIDIA (GTX 900/10 series).** Recommendation: unsupported in 1.0. The 580 branch is NVIDIA's last for those GPUs and will fall behind new kernels.

**U-10 — Team size and time budget.** Roadmap estimates assume one full-time engineer and are unvalidated.

**U-11 — Additional test hardware.** At least one RTX desktop and one AMD-GPU machine are needed before the hardware matrix can claim tested support beyond the reference laptop.

**U-12 — Disk encryption default.** Opt-in or default-on, with a passphrase in both cases; TPM unlock is excluded by D-030. Recommendation: opt-in for 1.0.

**U-13 — Legal review** of redistributing the NVIDIA driver, the Steam client and patent-encumbered codecs inside an image, and of Fedora trademark compliance. This needs a qualified person; nothing in these documents is legal advice.

### Added by the architecture review; decided inside a later phase

**U-14 — NVIDIA module flavour for Turing (GTX 16, RTX 20).** Open modules for every supported GPU (two variants, but Turing laptops may lose discrete-GPU power-down), or a third variant with closed modules for Turing. Decided in Phase 1 from R1 measurements: idle power draw and suspend behaviour under each flavour. The architecture supports either (D-032).

**U-15 — Core application set.** Which applications ship as RPMs in the image so that an offline install is usable (file manager, terminal, text editor, browser at minimum), and which are Flatpaks fetched when online. Decided in Phase 3.

**U-16 — Multimedia codec policy.** Fedora ships without patent-encumbered codecs and without hardware H.264/HEVC decoding. Shipping RPM Fusion's versions improves video playback and has legal implications (U-13). Decided in Phase 3.

**U-17 — Stable release cadence.** Bounded by how long a manual bare-metal run takes and by having one tester. Decided after Phase 1.

**U-18 — NVIDIA driver adoption policy.** Which upstream branch ARKON follows and how long a new driver soaks in `testing` before promotion. Decided in Phase 1.

### Raised by the Phase 2 design draft

**U-19 — Installer engine.** Approve D-034 (ARKON orchestrator over systemd-repart and bootc), or stay with Anaconda (D-012) and accept whole-disk installation only. Recommendation: D-034, staged, because the Windows and no-silent-erasure requirements cannot be met through Anaconda's current bootc path. This must be decided before Phase 2 implementation.

**U-20 — Same-disk dual boot in 1.0.** Recommendation: ship it only if its verification gate passes on a disposable Windows install; dual boot across two disks is the supported route regardless.

**U-21 — Live Plasma session or installer-only session on the medium.** Recommendation: live session, so users can check Wi-Fi, graphics and audio before installing.

**U-22 — Firmware boot order after a dual-boot install.** Recommendation: ARKON first, shown as a choice.

**U-23 — First-boot user creation.** ARKON handoff service with KDE's initial setup as fallback (recommended), or the KDE tool alone.

**U-24 — Recovery scope for 1.0.** Recommendation: boot repair; reinstall keeping home only if V-25 and V-26 pass.

**U-25 — A disposable Windows 11 installation for testing.** Required for every dual-boot and BitLocker criterion. The owner's working Windows installation must not be used. Needs a spare disk and a Windows licence or evaluation image.

**U-26 — Installer languages and accessibility for 1.0.** Recommendation: English first, with message codes so translation can follow.

**U-27 — "Restart into Windows" from the ARKON desktop.** Phase 4 candidate.
