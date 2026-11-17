# ARKON OS — Phase 2 Known Issues

**Date:** 2026-11-17
**Gate caveat:** Phase 1 is not approved and Phase 2 is not open. All items below were identified during the Phase 2 implementation at the owner's explicit request.

This file lists gaps, missing components, pre-existing warnings, and open questions identified during the Phase 2 engineering audit. Each item states its severity and what resolves it.

---

## Blocking — Phase 2 cannot ship

These items prevent any real installation from taking place. They are not code quality issues; they are absent components.

| ID | Item | Severity | Resolves in |
|---|---|---|---|
| KI-01 | Phase 1 (OS images, CI, signing, kernel modules) does not exist | **Fatal** | Phase 1 |
| KI-02 | `arkon-installd` D-Bus orchestrator does not exist | **Fatal** | Phase 2 |
| KI-03 | All `RealProvider` implementations in every crate return `Err("not implemented")` | **Fatal** | Phase 2 |
| KI-04 | No end-to-end installation has ever been run on any disk | **Fatal** | Phase 2 |
| KI-05 | Tauri frontend cannot be built on Windows (requires WebKitGTK on Linux) | **Fatal** | Phase 2 |
| KI-06 | No Linux live environment exists; `arkon-hwprobe` has never run on the target platform | **Fatal** | Phase 2 |
| KI-07 | Installation logging is not implemented in any crate | **Fatal** | Phase 2 |
| KI-08 | `systemd-repart` definitions are generated but have never been passed to `systemd-repart` | **Fatal** | Phase 2 |
| KI-09 | `bootc install to-filesystem` has never been called by any code in this repository | **Fatal** | Phase 2 |
| KI-10 | `bootupd` has never been called by any code in this repository | **Fatal** | Phase 2 |
| KI-11 | `mokutil` has never been called by any code in this repository | **Fatal** | Phase 2 |
| KI-12 | `useradd` and `hostnamectl` have never been called by any code in this repository | **Fatal** | Phase 2 |
| KI-13 | No git repository, no CI pipeline, no branch protection | **Fatal** | Phase 1 / Phase 2 |

---

## Non-blocking — Implementation gaps

These items are missing but do not prevent architecture validation or automated testing.

| ID | Item | Severity | Resolves in |
|---|---|---|---|
| KI-20 | Repair mode (re-create firmware entry, switch deployment) exists only in design docs; no frontend flow | High | Phase 2 |
| KI-21 | Installer accessibility not verified with a screen reader or keyboard-only walkthrough | High | Phase 2 |
| KI-22 | HW-B-01 through HW-B-09 (all physical hardware procedures) not run | High | Phase 2 |
| KI-23 | Dual-boot (HW-B-07) not tested; U-25 (disposable Windows install) unresolved | High | Phase 2 |
| KI-24 | Secure Boot state on the development machine is unknown (requires elevation to read) | Medium | Phase 2 |
| KI-25 | V-18 (files and entry label written by bootc + bootupd) unverified | Medium | Phase 2 |
| KI-26 | V-29 (Fedora shim compatibility with ASUS FA506IH firmware CA set) unverified | Medium | Phase 2 |
| KI-27 | V-12 (shim refuses a module-signing-only certificate) not tested on this hardware | Medium | Phase 1 / Phase 2 |
| KI-28 | V-22 (no BitLocker recovery prompt after MOK enrolment) unverified | Medium | Phase 2 |
| KI-29 | The offline repair mechanism for switching the default deployment (UEFI_BOOT_DESIGN.md §7) is unspecified | Medium | Phase 2 |
| KI-30 | `arkon-hwprobe` binary calls `RealProvider` on Linux but `RealProvider` is a stub returning errors | Low | Phase 2 |

---

## Pre-existing compiler warnings

These are unused-variable and unused-import warnings emitted by `cargo test`. They do not affect functionality and do not fail the build.

| Crate | File | Warning |
|---|---|---|
| arkon-storage | `src/planner.rs` | Unused import `PlannedPartition` |
| arkon-storage | `src/planner.rs` | Unused variables `esp_start`, `esp_size`, `boot_start`, `boot_size`, `root_start`, `root_size` in layout helper |
| arkon-storage | `src/validator.rs` | Unused imports `FreeRegion`, `RegionId`, `Transport` |
| arkon-storage | `tests/planner_integration.rs` | Unused imports `MIN_DISK_BYTES`, `MIN_FREE_REGION_BYTES`, `Transport`; unused `mut` on `disk`; unused `plan` variable |
| arkon-firstboot | `src/runner.rs` | Unused `journal` parameter in `execute_step` |
| arkon-installer | `tests/session_tests.rs` | Unused import `InstallMode`; unused `mut` on `s` |

**Cause:** The variables in `planner.rs` are placeholder positions for partition geometry that is computed but not yet consumed by the backend (which does not exist). They will be resolved when the backend is implemented.

**Recommendation:** `cargo fix` before the first backend implementation; do not suppress with `_` prefixes unless the backend is confirmed not to need those values.

---

## Design gaps not yet closed

| ID | Gap | Document |
|---|---|---|
| KI-40 | The handoff file path (`/run/arkon-installer/handoff.json` or similar) is not specified in any design document | PHASE2_ARCHITECTURE.md, to be added |
| KI-41 | The first-boot service unit name, `ExecStart`, and `Before=` ordering are not defined | Need a systemd unit file |
| KI-42 | `arkon-installd` D-Bus activation file and policy are not designed | Need a D-Bus service file and polkit policy |
| KI-43 | The ISO composition procedure (what goes in the ISO alongside the bootc image) is not specified for Phase 2 | V-24, V-25 open |
| KI-44 | Error messages shown to the user for each `FailureClass` are not written | Phase 2 UX work |
