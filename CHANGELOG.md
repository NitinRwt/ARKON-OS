# ARKON OS — Changelog

Newest first. Records changes to the repository, not plans.

## 2026-11-17 — Phase 1 image infrastructure and VM configurations

Added the full build infrastructure to produce bootable ARKON images and validate
them in virtual machines. No image has been built yet — Linux host and CI are required.

**New files — container image:**
- `image/Containerfile` — multi-stage bootc build: base → desktop → gaming → arkon / arkon-nvidia.
- `image/packages/{base,desktop,gaming,nvidia,removed}.txt` — per-stage RPM manifests.
- `image/repos/{rpmfusion-free,rpmfusion-nonfree}.repo` — RPM Fusion repository definitions.
- `image/rootfs/usr/lib/bootc/kargs.d/arkon-base.conf` — `quiet` kernel argument.
- `image/rootfs/usr/lib/bootc/kargs.d/arkon-nvidia.conf` — NVIDIA modeset / power management kargs.

**New files — build scripts:**
- `build/scripts/build-image.sh` — build ARKON container image with podman.
- `build/scripts/build-qcow2.sh` — produce QCOW2 via `quay.io/osbuild/image-builder --type qcow2`.
- `build/scripts/build-iso.sh` — produce installer ISO via `--type generic-iso`.
- `build/scripts/verify.sh` — SHA256 checksum verification for all artifacts.
- `build/scripts/clean.sh` — remove artifacts without destroying directory structure.

**New files — build configuration:**
- `build/config/image-builder-qcow2.toml` — osbuild blueprint (default VM user, services).
- `build/config/image-builder-iso.toml` — osbuild blueprint for Anaconda-based ISO.

**New files — VM configurations:**
- `vm/qemu/launch-arkon.sh` — QEMU/KVM launch: 4 vCPU, 8 GB RAM, UEFI, virtio, KVM, SSH forward.
- `vm/qemu/launch-arkon.bat` — Windows equivalent (no KVM; for testing a transferred QCOW2).
- `vm/qemu/validation-checklist.txt` — 21-item boot / desktop / system / NVIDIA checklist.
- `vm/vbox/arkon-vbox.sh` — VirtualBox create/start/delete (QCOW2 → VMDK conversion included).
- `vm/vmware/arkon.vmx` — VMware Workstation / Fusion VM definition (VMDK, UEFI, vmxnet3).
- `vm/vmware/convert-and-setup.sh` — qemu-img QCOW2 → VMDK conversion helper.

**New files — project root:**
- `Justfile` — `just` entry point: test, check, build-image, build-qcow2, build-iso, verify, qemu, clean, all.
- `.gitignore` — excludes artifacts (*.qcow2, *.vmdk, *.iso), VMware working files, Rust target/.

**Known limitations (recorded, not failures):**
- No image has been built. Linux host (podman, image-builder) is required.
- arkon-nvidia target blocked until `kmods/output/*.rpm` exist (kmods pipeline not yet built).
- `quay.io/osbuild/image-builder` container name may need updating post-bootc-image-builder merge.
- Boot validation results will be recorded here once a Linux build runs.

## 2026-11-17 — Storage scanner implementation

Implemented `arkon-storage` scanner — the previously missing entry point for
hardware-driven storage discovery used by the partition planner.

**New files:**
- `crates/arkon-storage/src/provider.rs` — `SysfsProvider` trait, `RealProvider`
  (reads /sys/block on Linux), `MockProvider` (in-memory fixture for tests).
- `crates/arkon-storage/src/scanner.rs` — `scan()` function:
  NVMe / SATA / USB / eMMC enumeration from `/sys/block`; partition type GUID
  classification; GPT vs MBR inference; free-region calculation; OS detection
  (ARKON, Windows, Linux); disk blockers and warnings; EFI firmware entry parsing
  (LABEL: mock format + binary UCS-2 path); install-medium detection.

**Modified:**
- `crates/arkon-storage/src/lib.rs` — exports `scan`, `SysfsProvider`,
  `RealProvider`, `MockProvider`.

**Tests:** 33 new unit tests in `scanner.rs`. Full suite: 185 tests, 0 failures
(previously 160; previous count in PROJECT_STATE.md was with mock-only coverage).

**Limitation:** `RealProvider` reads /sys/block on Linux only. Tests on this
Windows host use `MockProvider`. Real hardware verification deferred to Phase 2
gate on a Linux environment. EFI binary variable parsing is minimal
(description string only; no PARTUUID extracted from device path).

## 2026-11-17 — Phase 2 Final Engineering Audit

**Gate caveat:** Phase 1 not approved, Phase 2 not open. Audit conducted at owner's explicit request.

Audited all six Phase 2 crates, the HTML installer frontend, and the state documents across 17 evaluation areas. Ran the full test suite: 160 tests, 0 failures. Reviewed all RealProvider stubs, missing components, and unverified assumptions.

Verdict: **PHASE 2 NOT APPROVED.** Primary blockers:
1. Phase 1 (OS images, CI, signing, kernel modules) does not exist.
2. All `RealProvider` implementations return `Err("not implemented")`.
3. `arkon-installd` D-Bus orchestrator does not exist.
4. No hardware tests run. No end-to-end installation performed.
5. Dual-boot preservation not tested on real hardware.
6. Installation logging not implemented.
7. Tauri frontend not buildable on Windows host.

Added: [PHASE2_FINAL_AUDIT.md](PHASE2_FINAL_AUDIT.md), [PHASE2_KNOWN_ISSUES.md](PHASE2_KNOWN_ISSUES.md), [PHASE2_RELEASE_NOTES.md](PHASE2_RELEASE_NOTES.md).
Updated: [PHASE2_TEST_RESULTS.md](PHASE2_TEST_RESULTS.md) (added arkon-installer and arkon-firstboot test inventories), [PROJECT_STATE.md](PROJECT_STATE.md), [TASKS.md](TASKS.md).

## 2026-11-17 — Phase 2, Step 9: First-boot setup (arkon-firstboot)

**Gate caveat:** Phase 1 not approved, Phase 2 not open. Written at owner's explicit request.

Added `crates/arkon-firstboot` — restart-safe first-boot configuration and system health reporting.

- `error.rs` — `FirstBootError` with 11 typed variants; `FirstBootResult<T>`.
- `handoff.rs` — `HandoffData` (what `arkon-installd` writes at step X8): username, password hash, hostname, locale, keymap, timezone, language, MOK choice, variant, image reference; `validate()` enforces all field constraints.
- `state.rs` — `SetupStep` (11 ordered steps: CreateUser → ConfigureHostname → ConfigureTimezone → ConfigureNetwork → VerifyGraphicsDriver → VerifyNvidiaGpu → VerifyVulkan → VerifyAudio → VerifyUpdates → VerifyBootDeployment → GenerateHealthReport); `StepStatus` (Pending/Complete/Warning/Failed/Skipped); `SetupJournal` with JSON serialisation for restart-safe persistence.
- `health.rs` — Six typed health-check result types: `GraphicsDriverStatus`, `NvidiaStatus`, `VulkanStatus`, `AudioStatus`, `UpdateConfigStatus`, `BootDeploymentStatus`.
- `provider.rs` — `FirstBootProvider` trait (all host interactions abstracted); `MockProvider` for tests; `RealProvider` stub (Linux only).
- `runner.rs` — `FirstBootRunner`: loads or creates journal; skips completed steps; executes pending ones; persists journal after every step; deletes handoff file after successful run; builds health report.
- `report.rs` — `SystemHealthReport` assembled from all check results; `DiagnosticEntry` with `Severity` (Info/Warning/Error) sorted most-severe first; `all_ok` flag; `error_count()`/`warning_count()`.
- `lib.rs` — public API.

Safety properties enforced:
- Idempotent: user is not recreated if they already exist (Skipped status).
- Restart-safe: journal is written after every step; a re-run skips all completed steps.
- CreateUser is the only fatal step; all health checks produce warnings, never aborts.
- Handoff file is deleted after successful consumption (never re-applied).
- Provider trait enforces no arbitrary command passing.

Tests: **28 integration tests**, all pass. Workspace total: 160 tests, 0 failures.

## 2026-11-17 — Phase 2, Step 8: Installer library (arkon-installer, partial)

**Gate caveat:** Phase 1 not approved, Phase 2 not open. Written at owner's explicit request.

Added `crates/arkon-installer` — installer state machine, typed screen models, and configuration types.

- `error.rs` — `InstallerError` with 13 typed variants (`WrongState`, `PlanRejected`, `StalePlan`, `ConfirmationMismatch`, `TokenExpired`, `BitlockerAckRequired`, `NoPlan`, `PlanIdMismatch`, and others); `InstallerResult<T>`.
- `config.rs` — `Language`, `KeyboardLayout`, `UserConfig`, `SystemConfig` (with `validate()`); `MokChoice`; built-in language and keyboard layout lists.
- `progress.rs` — `ExecutionStep` (X1–X12 from §4 of the state machine spec) with labels and approximate progress fractions; `FailureClass` (F-PRE…F-POWER from §5); `DiskCondition`; `InstallFailure`; `InstallProgress`.
- `screens.rs` — `Screen` enum with one variant per installer screen (13 total: Welcome…Complete), each carrying typed state; `can_proceed()` enforces per-screen readiness rules; `InstallMode` (EraseDisk/FreeSpace/DualBoot/ReinstallKeepHome).
- `session.rs` — `SessionState` (12 states: Start…CleaningUp); `InstallerSession` with all transition methods (`begin_preflight`, `complete_preflight`, `block`, `rescan`, `create_plan`, `set_configuration`, `confirm`, `execute`, `advance_step`, `mark_verified`, `mark_complete`, `fail`, `begin_cleanup`, `finish_cleanup`, `cancel`); `ConfirmationToken` (single-use, bound to plan hash, injectable clock for deterministic tests).
- `lib.rs` — public API.

Safety rules enforced at the type level:
- S1: no method writes to disk before Confirmed.
- Confirmation token cannot be manufactured by the front end.
- Cancel is refused once X3 has started (Busy error).
- Destructive confirmation requires a matching phrase.
- BitLocker acknowledgement gate present.
- API rule 1: no method accepts commands, shell fragments, or image references.

Tests: **34 integration tests**, all pass. Workspace total: 132 tests, 0 failures.

Frontend (HTML SPA + Tauri shell): not yet implemented; see TASKS.md P2-13.

## 2026-11-17 — Phase 2, Step 7: UEFI and Secure Boot planning (arkon-boot)

**Gate caveat:** Phase 1 not approved, Phase 2 not open. Written at owner's explicit request.

Added `crates/arkon-boot` — planning-only boot configuration module; no firmware writes.

- `error.rs` — `BootError` with 11 typed variants; `SecureBootDisableRefused` documents the invariant that Secure Boot may never be disabled silently.
- `provider.rs` — `BootProvider` trait (`list_entries`, `boot_order`, `boot_next`, `esp_file_exists`, `list_esp_dir`); `MockProvider` for tests; `RealProvider` stub (Linux/sysfs, not yet implemented).
- `firmware.rs` — `FirmwareEntry`, `FirmwareSnapshot` (capture from provider); `compare_snapshots`, `validate_diff`, `find_stale_arkon_entries`; `DiffOutcome` distinguishes clean (one new entry) from violations.
- `esp.rs` — `EspFile` (Shim, Grub, FallbackLoader); `verify_esp`; `EspVerdict` (Ok / Incomplete).
- `secureboot.rs` — `SecureBootState` (On/Off/SetupMode/Unknown); `SecureBootRecommendation` with no `DisableSecureBoot` variant by design; `validate_secure_boot`.
- `mok.rs` — `MokEnrolmentState` (NotRequired/AlreadyEnrolled/SecureBootOff/Queued); `MokEnrolmentPlan`; `plan_mok_enrolment`; never executes `mokutil`.
- `bootloader.rs` — `Bootloader` (Grub2WithShim/SystemdBoot/Unknown); `validate_bootloader`; systemd-boot is `Unverified` not `Invalid` (D-003 spike path left open).
- `diagnostics.rs` — `DiagnosticFinding`; `RepairAction` with no `DisableSecureBoot` variant; `diagnose` function (read-only).
- `plan.rs` — `BootPlanRequest`, `BootPlan`, `build_boot_plan`; carries firmware before-snapshot, bootloader validation, Secure Boot report, MOK plan, warnings.
- `lib.rs` — public API.

Tests: **34 integration tests**, all pass. Workspace total: 98 tests, 0 failures.

Created [PHASE2_TEST_RESULTS.md](PHASE2_TEST_RESULTS.md): automated test inventory (34 tests) plus documented physical hardware test procedures HW-B-01…HW-B-09 (not yet run; require a Linux live image and physical access).

## 2026-11-17 — Phase 2, Step 5: storage and partition planning (arkon-storage)

**Gate caveat:** Phase 1 not approved, Phase 2 not open. Written at owner's explicit request.

Added `crates/arkon-storage` — planning-only engine, no disk writes.

- `error.rs` — `StorageError` with 14 typed variants covering every rejection reason.
- `scan.rs` — `StorageReport`, `Disk`, `Partition`, `FreeRegion`; rich classification types (`PartitionClass`, `DetectedSystem`, `DiskBlocker`, `DiskWarning`); builder helpers for tests.
- `plan.rs` — `PlanRequest`, `Plan`, `PlannedOperation`, `OperationType`, `RiskLevel`, `ValidationReport`; layout constants (`MIN_DISK_BYTES`=64 GiB, `ESP_SIZE_BYTES`=1 GiB, `BOOT_SIZE_BYTES`=2 GiB, `MIN_ROOT_BYTES`=15 GiB, `ALIGNMENT_BYTES`=1 MiB); GPT type GUIDs and ARKON partition labels.
- `planner.rs` — four planning modes: `EraseDisk`, `FreeSpace`, `Manual`, `ReinstallKeepHome`; FNV-1a plan hash; systemd-repart definition generation.
- `validator.rs` — nine validator rules P1–P9 from PARTITIONING_DESIGN.md §5; individual rule results returned so callers see exactly what failed.
- `dryrun.rs` — `DryRunReport` with per-step descriptions, reversibility markers, size summary and validation output; no disk access.
- `lib.rs` — public API: `plan()`, `plan_and_validate()`, `plan_dry_run()`.

Tests: **33 unit and integration tests**, all pass. Eight required scenarios covered plus extras. Workspace total: 64 tests, 0 failures.

## 2026-11-17 — Phase 2, Step 4: hardware and storage discovery (arkon-hw)

**Gate caveat:** Phase 1 is not approved and Phase 2 is not open. The implementation was written at the owner's explicit request. This does not change the gate status.

Added a Cargo workspace and two crates:

- `crates/arkon-hw` — hardware discovery library (task P2-10, partial).
  - `error.rs` — `ProbeError` / `ProbeResult` types.
  - `provider.rs` — `SysfsProvider` trait; `RealProvider` (reads /proc, /sys); `MockProvider` for unit tests.
  - `cpu.rs` — CPU vendor and model detection from `/proc/cpuinfo`; x86-64-v2 baseline check.
  - `gpu.rs` — GPU PCI detection from `/sys/bus/pci/devices/`; NVIDIA generation identification (Turing/Ampere/Ada); AMD and Intel GPU identification; static model name table.
  - `storage.rs` — NVMe and SATA device enumeration; per-partition metadata from sysfs (type GUIDs, fs type, UUID, label); EFI partition and OS detection (Windows, Linux, ARKON, LUKS).
  - `firmware.rs` — UEFI/legacy-BIOS detection; Secure Boot and Setup Mode state from efivars; 64/32-bit EFI detection.
  - `report.rs` — `HardwareReport` assembly; variant recommendation (arkon / arkon-nvidia / arkon-nvidia-turing); hardware warnings.
  - `lib.rs` — public API; `probe()` entry point.
- `crates/arkon-hwprobe` — CLI binary; prints the hardware report as JSON; accepts `--pretty`.

Tests: **31 unit tests**, all pass. Mocked via `MockProvider`; no host hardware access during tests.

No disk reads, no mounts, no shell commands, no privileged operations in this module.

## 2026-11-16 — Phase 2 design draft

Requested on the statement that Phase 1 had passed its audit. That statement does not match the record: the audit verdict is "not approved", and a re-inspection found the repository unchanged, with no Phase 1 implementation. The design was therefore written as a draft, Phase 2 was not opened, and no phase status was changed.

Added in `docs/phase2/`: PHASE2_ARCHITECTURE, INSTALLER_TECHNOLOGY_REVIEW, STORAGE_ARCHITECTURE, PARTITIONING_DESIGN, UEFI_BOOT_DESIGN, SECURE_BOOT_DESIGN, INSTALLATION_STATE_MACHINE, INSTALLER_API_SPEC, PHASE2_ACCEPTANCE_CRITERIA, PHASE2_TASKS.

Proposed: D-034…D-040. D-034 would replace Anaconda (D-012) with an ARKON orchestrator over systemd-repart and `bootc install to-filesystem`.
Raised: open decisions U-19…U-27; verification items V-17…V-31; risks R-35…R-46.
Edited: DECISIONS, RISK_REGISTER, PROJECT_STATE, TASKS, and pointers in ACCEPTANCE_CRITERIA and DEVELOPMENT_ROADMAP.
No implementation code written.

## 2026-11-16 — Phase 1 completion audit

Audited the repository against a report that Phase 1 had been implemented. Found no Phase 1 implementation: the repository holds only the Phase 0 documents, is not under version control, and no build has been run. Verdict: Phase 1 not approved.

Added: PHASE1_AUDIT.md, PHASE1_TEST_RESULTS.md, PHASE1_REMEDIATION.md.
Updated: PROJECT_STATE.md, TASKS.md, TEST_RESULTS.md.
No design document, decision or product file was changed.

## 2026-11-16 — Phase 0 architecture review

Reviewed the Phase 0 package for feasibility, bootc compatibility, NVIDIA/CUDA, AMD CPU support, gaming, Secure Boot, update and recovery, service boundaries, security and maintenance. No implementation code written.

Contradictions corrected:

- Container signature policy no longer defaults to reject; enforcement is scoped to the ARKON registry (D-026).
- "Pinned inputs" replaced by: base digest pinned and carrying the kernel, repository contents recorded (D-023).
- Variant schedules: explicit, time-limited divergence when the NVIDIA build fails (D-023).
- Stream and variant lists come from the signed image, not from the service binary (D-025).
- Diagnostics: file descriptor return; system journal content needs administrator authentication (D-025).
- Offline install versus Flatpak defaults: core applications are RPMs (U-15).
- One implementation of GPU detection: `arkon-hw` library and `arkon-hwprobe` CLI replace the `arkon-hwd` daemon (D-024).
- Updater split into an unprivileged D-Bus front and fixed root worker units (D-025).
- Fedora version tracks the current release instead of fixing 1.0 to Fedora 45 (D-031).
- Weekly stable cadence withdrawn; cadence undecided (U-17).
- Flatpak NVIDIA runtime kept in step with the host driver (D-028).

Design gaps closed:

- Module-signing certificate restricted to module signing (D-029).
- TPM automatic unlock and hibernation removed from 1.0 (D-030).
- NVIDIA module flavour made a build parameter after finding that open modules do not power down Turing GPUs (D-032, U-14).
- Freshness check against replay of old signed images (D-027).
- Bootloader update policy (D-033).
- Rollback limits for `/var` and home, and aborted-boot false positives, documented and made test cases.
- User-session agent added for notifications (D-025).
- AMD CPU support statement made specific; `amd-pstate` no longer assumed.
- Bare-metal test target recommendation revised; pre-install checklist for the Windows laptop added (U-01).

Added: decisions D-023…D-033; open decisions U-14…U-18; verification items V-11…V-16; risks R-26…R-34; acceptance criteria 0.9, 0.10, 1.14–1.18, 2.11, 2.12, 3.6, 4.11–4.15, 7.9–7.11, 9.9, 9.10.

Rewritten: SYSTEM_ARCHITECTURE, SECURITY_ARCHITECTURE, UPDATE_AND_RECOVERY_DESIGN, PROJECT_STATE, TASKS. Edited: all other documents.

Phase 0 gate still not passed: owner decisions U-01…U-07 and approval of D-001…D-033 are open.

## 2026-11-16 — Phase 0 architecture package

Added:

- Root state documents: PROJECT_STATE.md, ARCHITECTURE.md, DECISIONS.md, TASKS.md, CHANGELOG.md, TEST_RESULTS.md, RISKS.md.
- Design package in `docs/`: PRODUCT_VISION, SYSTEM_ARCHITECTURE, HARDWARE_SUPPORT_MATRIX, SECURITY_ARCHITECTURE, UPDATE_AND_RECOVERY_DESIGN, GAMING_ARCHITECTURE, AI_DEVELOPER_ARCHITECTURE, REPOSITORY_STRUCTURE, DEVELOPMENT_ROADMAP, ACCEPTANCE_CRITERIA, RISK_REGISTER.

Decided (proposed, awaiting owner approval): D-001 through D-022. One deviation from the charter's proposed stack: GRUB2 with Fedora's shim instead of systemd-boot for Phases 1–2 (D-003).

Not done: no implementation code, no git repository, no CI, no tests. Phase 0 gate not passed; blocking decisions U-01…U-07 are open.
