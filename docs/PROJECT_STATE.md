# ARKON OS — Project State

Last updated: 2026-11-17 (after Phase 2 Final Engineering Audit — verdict: PHASE 2 NOT APPROVED)
Read this file first in every session. It is the single entry point; everything else is linked from here.

## Current position

| Item | State |
|---|---|
| Phase | **0 — Product discovery and architecture** |
| Phase 0 gate | **Not passed.** Documents are written and reviewed. The owner must answer U-01…U-07 and approve or reject D-001…D-033. |
| Architecture review | Done 2026-11-16. Eleven contradictions and nine design gaps corrected; see CHANGELOG.md. |
| Phase 1 | **Not started.** A completion audit on 2026-11-16 found no Phase 1 implementation in this repository or elsewhere on this machine. Verdict: **not approved**. See [PHASE1_AUDIT.md](PHASE1_AUDIT.md). |
| Phase 2 | **NOT APPROVED.** See [PHASE2_FINAL_AUDIT.md](PHASE2_FINAL_AUDIT.md). Gate violation: the owner explicitly requested Phase 2 implementation before Phase 1 exists. All six Phase 2 crates were written at that request (2026-11-17). All `RealProvider` implementations are stubs; no `arkon-installd` orchestrator exists; no hardware tests have been run; dual-boot has not been tested; Tauri build requires Linux. The eleven Phase 1 blockers in [PHASE1_REMEDIATION.md](PHASE1_REMEDIATION.md) still stand. |
| Implementation code | `crates/arkon-hw`, `crates/arkon-hwprobe`, `crates/arkon-storage` (scanner implemented), `crates/arkon-boot`, `crates/arkon-installer`, `crates/arkon-firstboot` exist. No `arkon-installd` orchestrator, no service binary. |
| Image infrastructure | `image/Containerfile`, package manifests, RPM Fusion repo configs, kargs files exist. Build scripts in `build/scripts/`; VM configs in `vm/` (QEMU, VirtualBox, VMware). **No image has been built** — requires Linux host. `arkon-nvidia` variant blocked until kmods pipeline produces RPMs. |
| Repository | Plain directory, **not a git repository**, no remote, no CI. |
| Tests run | **185 unit/integration tests pass** (arkon-hw: 31, arkon-storage: 58, arkon-boot: 34, arkon-installer: 34, arkon-firstboot: 28; 2026-11-17, Windows / `cargo test`). **All tests use MockProvider. No real hardware was accessed.** See [TEST_RESULTS.md](TEST_RESULTS.md) and [PHASE2_TEST_RESULTS.md](PHASE2_TEST_RESULTS.md). |

**Phase 1 must not start until the gate passes.**

## What exists

- Root state documents: this file, [ARCHITECTURE.md](ARCHITECTURE.md), [DECISIONS.md](DECISIONS.md), [TASKS.md](TASKS.md), [CHANGELOG.md](CHANGELOG.md), [TEST_RESULTS.md](TEST_RESULTS.md), [RISKS.md](RISKS.md).
- Phase 0 package in [docs/](docs/): product vision, system architecture, hardware matrix, security, update and recovery, gaming, AI/developer, repository structure, roadmap, acceptance criteria, risk register.
- Phase 1 audit record: [PHASE1_AUDIT.md](PHASE1_AUDIT.md), [PHASE1_TEST_RESULTS.md](PHASE1_TEST_RESULTS.md), [PHASE1_REMEDIATION.md](PHASE1_REMEDIATION.md).
- Phase 1 image infrastructure (created 2026-11-17, unbuilt): `image/Containerfile`, `image/packages/`, `image/repos/`, `image/rootfs/`, `build/scripts/`, `build/config/`, `vm/qemu/`, `vm/vbox/`, `vm/vmware/`, `Justfile`, `.gitignore`.
- Phase 2 design draft in [docs/phase2/](docs/phase2/): start with [PHASE2_ARCHITECTURE.md](docs/phase2/PHASE2_ARCHITECTURE.md). It proposes D-034…D-040, raises U-19…U-27, V-17…V-31 and R-35…R-46, and must be revalidated once Phase 1 exists.
- Phase 2 audit record: [PHASE2_FINAL_AUDIT.md](PHASE2_FINAL_AUDIT.md), [PHASE2_TEST_RESULTS.md](PHASE2_TEST_RESULTS.md), [PHASE2_KNOWN_ISSUES.md](PHASE2_KNOWN_ISSUES.md), [PHASE2_RELEASE_NOTES.md](PHASE2_RELEASE_NOTES.md). Verdict: **not approved**.

Reporting rule adopted after the audit: a phase is called complete only when TEST_RESULTS.md holds, for each of its acceptance criteria, the image digest, the hardware, the date and the result. If Phase 1 work exists outside this directory, it has not been seen and must be brought here before it can be audited.

## Development host (inspected 2026-11-16)

| Item | Value |
|---|---|
| Machine | ASUS TUF Gaming A15 FA506IH |
| CPU | AMD Ryzen 5 4600H (Zen 2, 6 cores / 12 threads) |
| GPU | NVIDIA GeForce GTX 1650 (Turing, 4 GB) + AMD Radeon integrated (hybrid graphics) |
| RAM / storage | 15.4 GB / one 477 GB NVMe SSD (holds the Windows 11 install) |
| Firmware | UEFI. Secure Boot state not readable without elevation — unknown. |
| Windows disk encryption | Unknown — not checked (needs elevation). Must be checked before any install or firmware change. |
| OS | Windows 11 Home 10.0.26300 |
| Tooling present | git, Docker Desktop (installed, daemon stopped), WSL2 (only the `docker-desktop` distro), rustc/cargo, node/npm, Python 3.11 |
| Tooling absent | podman, gh, pytest, any Linux build host |

Consequences that shaped the design:

1. This laptop is the only known test hardware. It has **no RTX GPU**, so the charter's "NVIDIA RTX priority" cannot be validated on hardware the project owns today (R-01).
2. It is a hybrid-graphics laptop with a **Turing** GPU. NVIDIA's default open kernel modules are documented as unable to power that generation down at idle (R-29, U-14).
3. Its single disk holds Windows. Bare-metal testing must not touch that disk (U-01, R-03).
4. bootc images cannot be built natively on Windows. GitHub Actions is the build system of record (D-020).

## Architecture in ten lines

1. Fedora bootc image (Fedora 45 today, tracking the current release), built and signed in CI.
2. Variants by GPU: `arkon` (Mesa) and `arkon-nvidia` (prebuilt, signed modules); a closed-module Turing variant is possible.
3. The base image digest pins the kernel; modules are built for exactly that kernel.
4. Boot chain: Fedora shim → GRUB2 → Fedora kernel, Secure Boot on. systemd-boot deferred.
5. Updates: staged A/B deployments, signature and freshness checked, automatic rollback through greenboot.
6. Installer: Anaconda ISO built with image-builder; variant chosen by `arkon-hwprobe`.
7. Rust components: `arkon-hw` library and CLI, `arkon-updated` (unprivileged front plus fixed root workers), `arkon-secureboot`, `arkon-diag`, a session agent, `arkonctl`.
8. ARKON Center: Tauri front end over the same D-Bus interface.
9. Gaming: native Steam, Proton, Gamescope as a wrapper, MangoHud.
10. Development and AI: Podman containers, GPU through CDI, nothing installed on the host.

## Decisions the owner must make before Phase 1

Full text and recommendations are in [DECISIONS.md](DECISIONS.md#open-decisions).

| ID | Question | Recommendation |
|---|---|---|
| U-01 | Where does bare-metal testing install to? | A second internal drive if the laptop has a free slot, otherwise a dedicated external SSD; never the Windows disk |
| U-02 | GitHub organisation, repository visibility, registry namespace | Public repo, images on ghcr.io |
| U-03 | May CI boot images in QEMU as a secondary regression check? | Yes, secondary only; bare metal stays the acceptance target |
| U-04 | Accept GRUB2 + shim for Phases 1–2 instead of systemd-boot? | Yes, with a time-boxed systemd-boot spike in Phase 1 |
| U-05 | Accept RPM Fusion and Flathub as allow-listed third-party sources? | Yes, with pinned keys and recorded versions |
| U-06 | Steam as a native RPM in the image, or as a Flatpak? | Native RPM |
| U-07 | Project identity: reverse-DNS name, licence, trademark check on "ARKON" | Owner must supply |
| — | Approve or reject the proposed decisions D-001…D-033 | Approve; D-003 is the one deviation from the charter |

Needed before Phase 2 implementation, not before Phase 1: U-19 (installer engine: approve D-034 or keep Anaconda) and U-25 (a disposable Windows installation for dual-boot testing). The other Phase 2 questions, U-20…U-24, U-26 and U-27, are settled inside Phase 2.

Decisions that are deliberately made inside a later phase (not blocking): U-08 key custody, U-09 legacy NVIDIA, U-10 team size, U-11 more hardware, U-12 encryption default, U-13 legal review, U-14 Turing module flavour, U-15 core applications, U-16 codecs, U-17 stable cadence, U-18 driver adoption policy.

## Next actions

1. Owner answers the table above.
2. Record the answers in DECISIONS.md; revise any document an answer changes.
3. Re-run the Phase 0 gate in [docs/ACCEPTANCE_CRITERIA.md](docs/ACCEPTANCE_CRITERIA.md).
4. Before Phase 1's first install on the laptop: check Windows disk-encryption status and save the recovery key, back up, check for a free internal drive slot, read the Secure Boot state.
5. Only then start Phase 1 (task list is in [TASKS.md](TASKS.md); order of work is in [PHASE1_REMEDIATION.md](PHASE1_REMEDIATION.md)).
6. Request a new Phase 1 audit only when the re-audit criteria in PHASE1_REMEDIATION.md are met.

## Standing engineering rules (from the project charter)

- Workflow for every phase: Discovery → Requirements → Design → Risk analysis → Implementation plan → Implementation → Testing → Review → Documentation → Phase gate. A phase does not start until the previous gate passes.
- Never claim a feature works unless it was tested, or state the verification limit.
- No destructive disk operations, no unrestricted privileged commands, no default Secure Boot disabling, no telemetry without consent, no automatic overclocking, no unverified repositories in production builds, no toolchains in the base image.
- Bootloader, partitioning, kernel and driver changes require a written risk explanation first.
- Virtual machines are not the development or deployment target; physical hardware is.
- Architecture documents are changed only after the proposed correction has been explained.
- Response format per task: Objective, Existing state, Proposed design, Files affected, Implementation steps, Risks, Test plan, Acceptance criteria, Remaining work.
- At context limits: stop starting tasks, record done/outstanding/errors here, write a continuation summary.

## Unverified assumptions carried forward

Stated in the design, not confirmed by research. Each has a verification task in TASKS.md.

| ID | Assumption | Phase |
|---|---|---|
| V-01 | `quay.io/fedora/fedora-bootc:45` exists and a KDE Plasma package set installs cleanly on top of it | 1 |
| V-02 | With the ostree backend, bootc on x86_64 UEFI installs bootupd + GRUB2 + Fedora shim | 1 |
| V-03 | greenboot provides boot counting and health-check rollback for bootc on the GRUB chain; counter size and decrement point are suitable | 1 |
| V-04 | RPM Fusion's NVIDIA packaging can build open and closed kernel modules for a given kernel in a container | 1 |
| V-05 | An Anaconda kickstart can choose the `bootc` source image from a `%pre` hardware probe | 2 |
| V-06 | Every image variant fits in one ISO through shared container layers | 2 |
| V-07 | Which display manager Fedora KDE ships (SDDM or Plasma Login Manager) | 1 |
| V-08 | Current stable Tauri 2.x version; its WebKitGTK behaviour on NVIDIA + Wayland; continued availability of `webkit2gtk-4.1` in Fedora | 4 |
| V-09 | Fedora 45 end-of-life date (one source says 2027-12-08) | — |
| V-10 | Kernel arguments in `/usr/lib/bootc/kargs.d` are honoured on the chosen backend | 1 |
| V-11 | The NVIDIA variant reaches a usable desktop when its module cannot load. Only the hybrid-laptop case is testable on R1 | 1 |
| V-12 | Fedora's shim refuses an EFI binary signed with a module-only certificate | 1 |
| V-13 | Fedora-signed `kernel-devel` for the base digest's kernel is obtainable after the repositories have moved on | 1 |
| V-14 | How bootc signature enforcement is enabled through Anaconda's install path, and that `upgrade` and `switch` then honour it | 1, 2 |
| V-15 | `bootloader-update.service` is present and enabled in a `fedora-bootc`-derived image | 1 |
| V-16 | TensorFlow's current Linux GPU support and a maintained upstream GPU image | 6 |
| V-17…V-31 | Installer assumptions: what bootc and bootupd write to ESPs and firmware, partitioner behaviour beside existing partitions, Windows and BitLocker behaviour, ISO composition, handoff location. Listed in [docs/phase2/PHASE2_ARCHITECTURE.md](docs/phase2/PHASE2_ARCHITECTURE.md#verification-items) | 2 |
