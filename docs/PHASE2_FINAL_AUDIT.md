# ARKON OS — Phase 2 Final Engineering Audit

**Date:** 2026-11-17
**Auditor:** Claude Sonnet 4.6 (automated) — no independent human reviewer
**Gate caveat:** Phase 1 is not approved and Phase 2 was not formally opened. All implementation written at the owner's explicit request. This caveat must appear in every verdict document.

---

## Executive Summary

**Verdict: PHASE 2 NOT APPROVED**

The implementation written in Phase 2 is architecturally sound and all 160 automated tests pass. However, the system cannot be approved for any installation or distribution purpose because:

1. Phase 1 was never implemented or approved. The OS image, variant builds, kernel modules, signing infrastructure, and CI pipeline that Phase 2 installs do not exist.
2. All `RealProvider` implementations in every crate return `Err("not implemented")`. No crate performs a real operation on the target platform.
3. The D-Bus orchestrator (`arkon-installd`) does not exist. The installer library has no concrete backend.
4. The Tauri frontend cannot be built on the Windows development host (requires WebKitGTK on Linux).
5. No physical hardware tests have been run. All 160 tests use `MockProvider` and run on Windows.
6. Dual-boot preservation has not been tested on any real machine. Claims cannot be made about it.
7. The repository is not under version control, has no CI pipeline, and no image has ever been built.

Specific blocking items are listed by area in §3 and compiled in PHASE2_KNOWN_ISSUES.md.

---

## Scope

This audit covers the complete bare-metal installation system: the six Rust crates written in Phase 2 (`arkon-hw`, `arkon-hwprobe`, `arkon-storage`, `arkon-boot`, `arkon-installer`, `arkon-firstboot`), the HTML installer frontend, and the state documents. It does not audit Phase 0 or Phase 1 documents, which were audited separately.

---

## §1 — What was tested

### Automated tests — PASSED

All tests run on Windows 11 / `cargo test` using `MockProvider` only. No host OS, firmware, disk, or network access during any test.

| Crate | Tests | Passed | Failed |
|---|---|---|---|
| arkon-hw | 31 | 31 | 0 |
| arkon-storage | 33 | 33 | 0 |
| arkon-boot | 34 | 34 | 0 |
| arkon-installer | 34 | 34 | 0 |
| arkon-firstboot | 28 | 28 | 0 |
| **Total** | **160** | **160** | **0** |

Detailed test inventory in PHASE2_TEST_RESULTS.md.

### Automated tests — NOT run / NOT applicable

| Test type | Reason not run |
|---|---|
| `arkon-hwprobe` binary (bare-metal probe) | Requires a Linux live session; cannot run on Windows |
| Tauri frontend build | Requires WebKitGTK; not available on Windows |
| D-Bus IPC round-trip | `arkon-installd` does not exist |
| Installation dry-run end-to-end | No backend; no image; no orchestrator |
| All hardware procedures HW-B-01 to HW-B-09 | No Linux live image; no test disk |

### Manual tests — NOT run

No manual testing of any kind has been performed. The installer frontend cannot be driven without an orchestrator.

### Hardware tests — NOT run

No hardware test has been performed. The development machine (ASUS TUF A15 FA506IH, Windows 11 Home, single NVMe) has not been booted from any ARKON medium.

---

## §2 — What exists vs. what is missing

### Exists (automated, partial)

| Component | State |
|---|---|
| `arkon-hw` — hardware discovery types, MockProvider | Complete; RealProvider is a stub |
| `arkon-hwprobe` — CLI binary | Compiles; calls `RealProvider` which returns errors on Linux |
| `arkon-storage` — partition planner, validator, dry-run | Logic complete; no disk I/O anywhere |
| `arkon-boot` — UEFI/SB planning types | Planning only; no firmware writes |
| `arkon-installer` — 12-state session, 13 screen models | State machine complete; no D-Bus backend |
| `arkon-firstboot` — 11-step restart-safe runner | Runner logic complete; RealProvider is a stub |
| HTML installer frontend | All 13 screens; mock progress simulation |
| Tauri shell stub | Compiles with feature-gate; Tauri dep commented out |

### Does not exist (blockers for any real installation)

| Component | Impact |
|---|---|
| `arkon-installd` D-Bus orchestrator | The installer library has no backend; the frontend cannot call anything real |
| All `RealProvider` implementations | Every crate's production path returns `Err("not implemented")` |
| bootc image (any variant) | Nothing to install |
| Installation ISO | Nothing to boot from |
| systemd-repart integration | Partition operations are planned but never executed |
| `bootc install to-filesystem` integration | Deployment step never runs |
| `bootupd` integration | Firmware writes never execute |
| `mokutil` integration | MOK enrolment never executes |
| `useradd` / systemd integration | First-boot user creation never runs |
| Installation logging to journal | No log output from any real operation |
| Phase 1 (images, CI, signing, modules) | Required foundation; does not exist |

---

## §3 — Evaluation by area

### Area 1: Installation reliability

**Result: CANNOT EVALUATE — no installation has ever run.**

The installer state machine is well-designed: 12 states, single-use confirmation token, plan hash, and cancel gating at X3. Safety rules S1–S8 are enforced at the type level. All state transitions are tested (34 tests).

No end-to-end installation has been run. The backend that executes the plan (`arkon-installd`) does not exist. The execution steps X1–X12 have no implementations behind them.

**Blocking:** `arkon-installd` must be built and an end-to-end installation must succeed on a real disk before this area can be evaluated.

---

### Area 2: UEFI compatibility

**Result: CANNOT EVALUATE — no hardware test run.**

The boot planning module (`arkon-boot`) correctly models the UEFI concepts: firmware entry snapshot/diff, ESP file verification, vendor directory `EFI/fedora`, single new entry invariant, and stale-entry detection. All 34 boot tests pass using MockProvider.

Verification items V-18 (exact files and entry label written by bootc/bootupd) and V-29 (shim compatibility with the ASUS firmware's CA set) are open.

**Blocking:** HW-B-03 and HW-B-04 (firmware entry preservation and ESP contents on hardware) have not been run.

---

### Area 3: Secure Boot

**Result: CANNOT EVALUATE — no hardware test run.**

Design is correct: Secure Boot off is shown but not refused (confirmed by test `secure_boot_off_shown_not_refused`). `DisableSecureBoot` is intentionally absent from all recommendation and repair enums. Plan carries SB state and MOK enrolment plan.

Secure Boot state on the development machine is unknown (requires elevation to read on Windows 11).

V-12 (Fedora shim refuses a module-signing-only certificate) is unverified. V-29 (shim compatibility with this machine's firmware) is unverified.

**Blocking:** HW-B-05 (Secure Boot end-to-end) has not been run. Certificate infrastructure (Phase 1, P1-08) does not exist.

---

### Area 4: NVIDIA driver integration

**Result: CANNOT EVALUATE — no image, no hardware test.**

Hardware detection correctly classifies the GTX 1650 as Turing (`arkon_nvidia_turing` variant recommendation). MOK planning correctly queues enrolment when Secure Boot is on and no NVIDIA certificate is enrolled. All relevant tests pass.

The `arkon-nvidia-turing` image variant does not exist. No kernel modules have been built or signed. The MOK enrolment flow (HW-B-06) has not been run. NVIDIA driver loading on the development machine's GTX 1650 has not been verified.

**Blocking:** Phase 1 P1-07 (kmods pipeline), P1-08 (module-signing cert), P1-09 (NVIDIA stage) must be built and tested before this area can be evaluated.

---

### Area 5: AMD CPU detection

**Result: PARTIALLY TESTED (automated only).**

`arkon-hw` correctly parses AMD CPU entries from `/proc/cpuinfo` mock data. The x86-64-v2 baseline check is tested and passes for the Ryzen 5 4600H fixture. The `arkon_vendor_detected` and `amd_pstate` driver details are modelled.

`arkon-hwprobe` has not been run on the development machine. Assumption V-02 (bootc on x86_64 UEFI installs with the right boot chain) is unverified for AMD CPUs specifically.

**Not blocking** for the architecture, but HW-B-02 (hardware probe accuracy on the real machine) has not been run.

---

### Area 6: NVMe and SATA handling

**Result: PARTIALLY TESTED (automated only).**

`arkon-storage` correctly models NVMe and SATA disks. Scenario tests cover NVMe disks at 200 GiB (1024-byte sectors) and standard 512-byte sectors. 4K sector disks are accepted; unsupported sector sizes are rejected.

`arkon-hw`'s storage module correctly reads NVMe (`nvme`/`nvme_port` sysfs entries) and SATA (`ata`/`host`) from mock sysfs data.

No real disk has been scanned. `SysfsProvider::RealProvider` returns `Err("not implemented")` for all storage calls.

---

### Area 7: Automatic partitioning

**Result: PARTIALLY TESTED (automated only).**

The `EraseDisk` planner produces correct GPT layouts: ESP (1 GiB), /boot (2 GiB), root (remainder). All constants are defined (`ESP_SIZE_BYTES`, `BOOT_SIZE_BYTES`, `MIN_ROOT_BYTES`, `MIN_DISK_BYTES`). Nine validator rules P1–P9 enforce safety invariants. The dry-run report lists each operation with reversibility markers.

25 integration test scenarios cover empty disk, Windows disk, Linux disk, LUKS disk, install medium, LVM, MBR, multiple NVMe drives, insufficient storage, 4K sectors, and dual-boot free-space.

No real partition has ever been created. `systemd-repart` definitions are generated by the planner but have never been passed to `systemd-repart`. No execution path exists in the codebase.

---

### Area 8: Manual partitioning

**Result: PARTIALLY TESTED (automated only).**

`Manual` mode is implemented in the planner and tested (scenario_manual_root_size_respected, scenario_manual_root_too_small_rejected). The installer frontend has a Partition Configuration screen with manual mode support.

Manual partitioning has never been executed against a real disk.

---

### Area 9: Windows dual-boot preservation

**Result: CANNOT EVALUATE — not tested on real hardware. No claim is made.**

Safety rule S3 (never shrink, move or reformat existing partitions) and S4 (never write to an ESP not created this run) are enforced at the type level. The firmware-entry diff invariant ensures Windows Boot Manager is never removed or changed.

The `FreeSpace` planner and `scenario_dual_boot_free_space_keeps_all_existing_partitions` test verify that existing partitions appear in the partition table unchanged.

**However:** no test has been run on a real machine with a real Windows installation. U-25 (a disposable Windows installation for dual-boot testing) is unresolved. HW-B-07 has not been run.

**No claim is made that dual-boot with real Windows data is safe.** This area requires physical testing on a disposable Windows installation before any statement can be made.

---

### Area 10: Bootloader installation

**Result: CANNOT EVALUATE — no end-to-end run.**

The boot plan correctly specifies the GRUB2+shim chain, including the correct vendor directory path (`EFI/fedora`), fallback loader, and firmware entry. `bootupd` integration is specified in the design.

`bootupd` has not been called by any code in this repository. The integration point exists only as a step in the state machine (`X7FirmwareEntry`). The `RealProvider::write_firmware_entry` method does not exist.

---

### Area 11: User account creation

**Result: PARTIALLY TESTED (automated only).**

`arkon-firstboot` correctly models user creation: idempotent (skipped if user already exists), fatal step (run aborts if `CreateUser` fails), validated username/hostname rules, and journal persistence after each step.

28 integration tests cover these scenarios using `MockProvider`. `RealProvider::create_user` returns `Err("not implemented")`. No real user has been created on a Linux system by this code.

---

### Area 12: First boot configuration

**Result: PARTIALLY TESTED (automated only).**

All 11 setup steps are modelled, restart-safe, and tested. Health check results (graphics driver, NVIDIA, Vulkan, audio, update config, boot deployment) feed a structured diagnostic report. Non-fatal steps produce warnings rather than aborting the run.

The handoff file (written by the installer at step X8) is consumed and deleted by the first-boot runner. All of this logic is correct per the tests.

No first-boot run has executed on a real Linux system. All 11 `RealProvider` methods return `Err("not implemented")`.

---

### Area 13: Recovery after failed installation

**Result: PARTIALLY TESTED (design and type level only).**

The installer's `FailureClass` enum defines 10 failure classes (F-PRE through F-POWER). Each failure leaves a stated `DiskCondition` and a next recommended step. `cleanup_offered()` is correct per class. The session state machine transitions through `Failed → CleaningUp → (Scanned|Failed)` correctly.

The repair mode described in UEFI_BOOT_DESIGN.md §7 (re-creating a missing firmware entry, switching deployment) has not been built as a user-facing flow. HW-B-08 and HW-B-09 (repair mode on hardware) have not been run.

---

### Area 14: Security and privilege boundaries

**Result: DESIGN VERIFIED — implementation not tested end-to-end.**

The privilege boundary is correctly specified:
- `arkon-installer` (Tauri, unprivileged) only calls typed D-Bus methods — no shell fragments, no image references, no mount options.
- API rule 1 is enforced: no method accepts a command string from the caller.
- API rule 4 (secrets cross as file descriptors, never as message arguments) is in the spec.
- S1 (no writes before Confirmed) is enforced by the session state machine's `writes_possible()` gate.
- S6 (no caller-supplied commands) is enforced at the type level in all crates.

The D-Bus service (`arkon-installd`) does not exist, so the boundary has not been exercised in practice. The Tauri CSP in `tauri.conf.json` is correctly set (`default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'`).

---

### Area 15: Installation logging

**Result: NOT IMPLEMENTED.**

No logging infrastructure exists in any crate. The design specifies that the installer writes to the system journal, but no `tracing` or `log` crate is present in any `Cargo.toml`. Failure information is captured in the `InstallFailure` struct but is never persisted to disk.

**Blocking:** Logging must be implemented before any real installation can be diagnosed.

---

### Area 16: Installer accessibility

**Result: PARTIALLY ADDRESSED — not verified.**

The HTML frontend uses semantic HTML (`<fieldset>`, `<legend>`, `<label for>`), visible focus states, and keyboard navigation. Screen elements are labelled. Dark/light theme toggle is present.

No accessibility audit has been run with a screen reader (NVDA/Orca). No keyboard-only walkthrough has been verified on the actual Tauri build. The Tauri build does not exist on the development host.

---

### Area 17: Regression tests

**Result: 160 automated tests exist and all pass. Hardware regression tests do not exist.**

The 160 tests cover state machine transitions, screen gate rules, partition planning scenarios, boot planning, first-boot runner, and health check logic. They form a sound regression baseline for the pure-logic layers.

No regression tests exist for:
- Real disk operations
- Real firmware writes
- Real D-Bus IPC
- Real systemd unit execution
- Real NVIDIA module loading
- Any end-to-end scenario

---

## §4 — Safety rules audit

| Rule | Enforcement | Tested |
|---|---|---|
| S1: no writes before Confirmed | `writes_possible()` returns false for all pre-Executing states | Yes — `no_state_before_executing_has_writes_possible`, `executing_state_has_writes_possible` |
| S2: destructive plan needs hash + phrase | `confirm()` enforces phrase + hash match; token is single-use | Yes — `confirm_correct_phrase_accepted`, `confirm_wrong_phrase_rejected`, `confirm_hash_mismatch_rejected` |
| S3: never shrink/move/reformat existing partitions | `EraseDisk` only creates new partitions; `FreeSpace` never touches existing | Yes — `scenario_dual_boot_free_space_keeps_all_existing_partitions` |
| S4: never write to ESP not created this run | `validate_diff` and ESP write gate | Partial — type-level enforcement; not tested end-to-end |
| S5: never delete/edit existing firmware entries | `validate_diff` returns violation if existing entry changed/removed | Yes — `diff_violation_entry_changed`, `diff_violation_entry_removed` |
| S6: no caller-supplied commands | No method in any crate accepts a string as a command | Yes — API design enforced at type level |
| S7: encrypted/unknown partitions never auto-selected | `BitlockerAckRequired` gate; LUKS classified as warning | Yes — `confirm_bitlocker_ack_required`, `scenario_bitlocker_disk_requires_acknowledgement` |
| S8: every failure leaves stated disk condition + next step | `InstallFailure` carries `DiskCondition` + `FailureClass` | Yes — `fail_during_execution` |

---

## §5 — Open decisions and verification items

The following remain open and must be resolved before Phase 2 can be re-audited:

| ID | Item | Blocking |
|---|---|---|
| U-25 | Disposable Windows installation for dual-boot testing | Area 9 |
| V-18 | Exact files and entry written by bootc + bootupd | Area 2, 10 |
| V-19 | bootupd behaviour with two ESPs on one disk | Area 10 |
| V-21 | Firmware boots from a second ESP on this hardware | Area 2, 9 |
| V-22 | No BitLocker recovery prompt after MOK enrolment | Area 4, 9 |
| V-29 | Fedora shim on ASUS FA506IH firmware CA set | Area 2, 3 |
| V-31 | bootc signature enforcement with scoped container policy | Area 1, 3 |

---

## §6 — Verdict

**PHASE 2 NOT APPROVED**

Reasons:

1. **Phase 1 prerequisite unmet.** The OS image, CI pipeline, kernel modules, and signing infrastructure that Phase 2 installs do not exist. No corrective action within Phase 2 can resolve this.

2. **No real operations implemented.** All `RealProvider` stubs return errors. The installer library, first-boot runner, and hardware probe have never performed a real operation.

3. **No orchestrator.** `arkon-installd` (the D-Bus backend) does not exist. The typed APIs cannot be called by anything real.

4. **No hardware tests.** All 160 tests use `MockProvider` on Windows. Procedures HW-B-01 through HW-B-09 have not been run.

5. **Dual-boot not tested.** No claim is made about dual-boot preservation on real hardware. U-25 is unresolved.

6. **No logging.** No log output is written by any crate.

7. **Frontend not buildable.** The Tauri build requires a Linux host with WebKitGTK.

**Re-audit criteria** (not binding — the owner may add more):

1. Phase 1 passes its re-audit and all eleven Phase 1 blockers in PHASE1_REMEDIATION.md are resolved.
2. All `RealProvider` methods in `arkon-hw`, `arkon-storage`, `arkon-boot`, `arkon-installer`, and `arkon-firstboot` are implemented and can run on a live Linux system.
3. `arkon-installd` exists, starts as a privileged D-Bus service, and handles all Tauri commands.
4. The Tauri application builds on a Linux host.
5. At least one clean-install end-to-end test on physical hardware succeeds, with the result recorded in PHASE2_TEST_RESULTS.md (image digest, hardware, date, outcome).
6. HW-B-01 through HW-B-06 pass on the development machine's test disk.
7. Installation logging writes to the system journal throughout execution.
8. If U-25 is resolved: HW-B-07 passes, and dual-boot preservation can then be claimed.
