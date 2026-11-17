# ARKON OS — Phase 2 Test Results

**Gate caveat:** Phase 1 is not approved and Phase 2 is not open. All entries in this file were produced at the owner's explicit request (2026-11-17). Tests that require a Linux live environment or physical hardware are recorded as "not run" until they are actually performed.

---

## Summary

| Area | State |
|---|---|
| Hardware probe tests (arkon-hw) | **31 passed, 0 failed** — 2026-11-17, Windows host, `cargo test` |
| Storage planner tests (arkon-storage) | **33 passed, 0 failed** — 2026-11-17, Windows host |
| Automated boot configuration tests (arkon-boot) | **34 passed, 0 failed** — 2026-11-17, Windows host, `cargo test` |
| Installer state machine tests (arkon-installer) | **34 passed, 0 failed** — 2026-11-17, Windows host, `cargo test` |
| First-boot runner tests (arkon-firstboot) | **28 passed, 0 failed** — 2026-11-17, Windows host, `cargo test` |
| **Workspace total** | **160 passed, 0 failed** |
| Bare-metal boot tests | **Not run** — no Linux live image exists |
| Firmware write tests | **Not run** — no installer orchestrator exists |
| ESP verification on hardware | **Not run** — requires a Linux live session |
| MOK enrolment flow | **Not run** — requires a physical console session |
| End-to-end installation | **Not run** — `arkon-installd` does not exist |
| First-boot on Linux | **Not run** — all RealProviders are stubs |

---

## 2026-11-17 — arkon-boot automated configuration tests

Host: Windows 11 / `cargo test`.  All tests use `MockProvider`; no firmware, EFI variables, or ESP are accessed.

| Suite | Tests | Passed | Failed |
|---|---|---|---|
| Firmware snapshot and diff | 7 | 7 | 0 |
| ESP contents verification | 4 | 4 | 0 |
| Secure Boot state | 4 | 4 | 0 |
| MOK enrolment planning | 4 | 4 | 0 |
| Bootloader validation | 3 | 3 | 0 |
| Repair-mode diagnostics | 5 | 5 | 0 |
| Boot plan builder | 7 | 7 | 0 |
| **arkon-boot total** | **34** | **34** | **0** |

### Test inventory

| Test | Scenario | Result |
|---|---|---|
| `snapshot_records_all_entries` | FirmwareSnapshot captures all entries and boot order | pass |
| `diff_clean_arkon_added` | Exactly one new ARKON entry — clean diff | pass |
| `diff_violation_entry_changed` | Existing entry label changed — violation detected | pass |
| `diff_violation_entry_removed` | Existing entry removed — violation detected | pass |
| `diff_violation_multiple_new_entries` | Two new entries — violation detected | pass |
| `windows_entry_preserved_in_diff` | Windows Boot Manager not in removed or changed | pass |
| `stale_arkon_entry_detected` | ARKON label + wrong UUID → stale entry found | pass |
| `esp_all_files_present` | shim, grub, fallback all present → Ok | pass |
| `esp_shim_missing` | shim absent → Incomplete(1) | pass |
| `esp_grub_missing` | grub absent → Incomplete(1) | pass |
| `esp_fallback_loader_missing` | fallback absent → Incomplete(1) | pass |
| `secure_boot_on_no_nvidia_no_action` | SB on, no NVIDIA → NoAction | pass |
| `secure_boot_off_shown_not_refused` | SB off → NoAction (not an error) | pass |
| `secure_boot_on_nvidia_requires_mok` | SB on + NVIDIA → EnrolArkonCert | pass |
| `setup_mode_state_is_recorded` | SetupMode state reported faithfully | pass |
| `mok_required_nvidia_secure_boot_on` | NVIDIA + SB on + not enrolled → Queued | pass |
| `mok_skip_secure_boot_off` | NVIDIA + SB off → SecureBootOff | pass |
| `mok_skip_already_enrolled` | NVIDIA + SB on + enrolled → AlreadyEnrolled | pass |
| `mok_not_required_no_nvidia` | No NVIDIA → NotRequired | pass |
| `grub2_shim_is_valid` | GRUB2+shim → Valid, no warnings | pass |
| `systemd_boot_is_unverified` | systemd-boot → Unverified + SystemdBootNotVerified warning | pass |
| `unknown_bootloader_is_invalid` | Unknown bootloader → Invalid | pass |
| `diagnose_missing_firmware_entry` | No ARKON entry → FirmwareEntryMissing + RecreateFirmwareEntry | pass |
| `diagnose_mok_not_enrolled` | Queued MOK state → MokNotEnrolled + QueueMokEnrolment | pass |
| `diagnose_previous_deployment_available` | 3 deployments → PreviousDeploymentAvailable(2) | pass |
| `diagnose_stale_arkon_entry` | Stale entry passed in → StaleArkonEntry + RemoveStaleEntry | pass |
| `diagnose_healthy_system` | Correct entry + clean ESP + no MOK issue → healthy=true | pass |
| `plan_clean_install_grub2` | GRUB2+shim, SB on → valid plan, correct loader path | pass |
| `plan_windows_entry_preserved_in_snapshot` | Windows entry in before-snapshot | pass |
| `plan_nvidia_sb_on_queues_mok` | NVIDIA + SB on → MOK Queued state in plan | pass |
| `plan_nvidia_sb_off_skips_mok` | NVIDIA + SB off → MOK SecureBootOff state | pass |
| `plan_systemd_boot_produces_warning` | systemd-boot → UnverifiedBootloader warning (not error) | pass |
| `plan_secure_boot_off_carries_warning` | SB off → SecureBootOff warning in plan | pass |
| `plan_stale_arkon_entry_produces_warning` | Stale ARKON entry → StaleArkonEntryDetected warning | pass |

---

## Physical Hardware Test Procedure

**Status: Not run.** Prerequisites are listed below. Tests marked (V-xx) correspond to unverified assumptions in PROJECT_STATE.md. These tests require the owner at the machine and must not be run until Phase 1 and Phase 2 pass their gates.

**Machine:** ASUS TUF Gaming A15 FA506IH (AMD Ryzen 5 4600H, GTX 1650, 477 GB NVMe, Windows 11 Home)

### Prerequisites

Before any bare-metal test on the development machine:

1. Check Windows disk-encryption status and **save the recovery key** (Settings → Accounts → Sign-in options → BitLocker; or Microsoft account at account.microsoft.com/devices/recoverykey).
2. Verify a free internal drive slot or acquire a dedicated external SSD for the ARKON install target (U-01). **Never install to the Windows NVMe.**
3. Obtain the firmware boot menu key for this model (Del or F2 for ASUS). Note the UEFI Secure Boot state in firmware setup (the Windows host cannot read it without elevation).
4. Build an ARKON installation ISO (Phase 1 and Phase 2 prerequisite — does not exist yet).
5. Write the ISO to a USB stick and verify its signature and checksum.

---

### HW-B-01 — UEFI boot of the installation medium

| Step | Action | Expected | Actual | Date | Tester |
|---|---|---|---|---|---|
| 1 | Insert USB medium and boot from it via the firmware boot menu | Medium boots in UEFI mode (not BIOS/CSM) | — | — | owner |
| 2 | Confirm shim→GRUB chain loads | GRUB menu appears with no Secure Boot error | — | — | owner |
| 3 | Note Secure Boot state shown by the installer welcome screen | On / Off / SetupMode | — | — | owner |

---

### HW-B-02 — Hardware detection accuracy (V-05, arkon-hwprobe)

| Step | Action | Expected | Actual | Date | Tester |
|---|---|---|---|---|---|
| 1 | From the live environment, run `arkon-hwprobe --pretty` | JSON output includes AMD CPU (Ryzen 5 4600H), NVIDIA GTX 1650 (Turing), one NVMe SSD | — | — | owner |
| 2 | Confirm variant recommendation is `arkon-nvidia-turing` | `recommended_variant: "arkon_nvidia_turing"` | — | — | owner |
| 3 | Confirm x86-64-v2 baseline check passes | `x86_64_v2: true` | — | — | owner |

---

### HW-B-03 — Firmware entry snapshot and preservation (V-18)

| Step | Action | Expected | Actual | Date | Tester |
|---|---|---|---|---|---|
| 1 | Before deployment, record all EFI boot entries with `efibootmgr -v` | Minimum: Windows Boot Manager entry exists | — | — | owner |
| 2 | Complete an ARKON installation to the test target | Installation succeeds | — | — | owner |
| 3 | After deployment, run `efibootmgr -v` again | Exactly one new entry "ARKON OS" pointing at shim on the ARKON ESP; Windows Boot Manager entry unchanged | — | — | owner |
| 4 | Compare entry numbers, labels, and partition UUIDs | No pre-existing entry removed or modified | — | — | owner |

This test validates the firmware-entry preservation invariant from UEFI_BOOT_DESIGN.md §3.  If step 3 or 4 fails, the installation must report the violated entry with its before-state; the test fails.

---

### HW-B-04 — ESP contents verification (V-18)

| Step | Action | Expected | Actual | Date | Tester |
|---|---|---|---|---|---|
| 1 | Mount the ARKON ESP and list `EFI/fedora/` | `shimx64.efi` and `grubx64.efi` present | — | — | owner |
| 2 | Check `EFI/BOOT/` | `BOOTX64.EFI` present (fallback loader) | — | — | owner |
| 3 | Verify vendor directory is `EFI/fedora` | Correct — matches the signed shim's built-in path | — | — | owner |

---

### HW-B-05 — Secure Boot state shown, never disabled (charter requirement)

| Step | Action | Expected | Actual | Date | Tester |
|---|---|---|---|---|---|
| 1 | With Secure Boot ON, run the installer to completion | Installer shows Secure Boot state; never presents "disable Secure Boot" as a step | — | — | owner |
| 2 | Reboot; verify system boots under Secure Boot | System boots without Secure Boot prompt or error | — | — | owner |
| 3 | With Secure Boot OFF, run the installer | Installer notes that SB is off; proceeds; does not ask the user to do anything about SB | — | — | owner |

---

### HW-B-06 — MOK enrolment flow (NVIDIA variant, V-12; SECURE_BOOT_DESIGN.md §5)

| Step | Action | Expected | Actual | Date | Tester |
|---|---|---|---|---|---|
| 1 | Install the `arkon-nvidia-turing` variant with Secure Boot ON | Final installation screen shows the one-time MOK password and a picture of the enrolment screen | — | — | owner |
| 2 | Reboot | shim's key manager (MokManager) appears at the console | — | — | owner |
| 3 | Select Enroll MOK → Continue; enter the password | Certificate enrolled; MokManager exits | — | — | owner |
| 4 | Boot completes; check NVIDIA driver status | `nvidia-smi` succeeds; module loaded | — | — | owner |
| 5 | Reboot again | System boots without MokManager prompt; NVIDIA driver loads | — | — | owner |

---

### HW-B-07 — Windows Boot Manager preservation (dual-boot; U-25 required)

Prerequisite: a **disposable** Windows installation on the test target (U-25 — not available yet).

| Step | Action | Expected | Actual | Date | Tester |
|---|---|---|---|---|---|
| 1 | Before installation, record the Windows Boot Manager entry | Label, partition UUID, loader path noted | — | — | owner |
| 2 | Install ARKON to a second partition or disk | Installation succeeds | — | — | owner |
| 3 | Verify the firmware boot menu shows both entries | "ARKON OS" and "Windows Boot Manager" present | — | — | owner |
| 4 | Select Windows Boot Manager from the firmware menu | Windows boots normally | — | — | owner |
| 5 | Select ARKON OS from the firmware menu | ARKON boots normally | — | — | owner |

---

### HW-B-08 — Repair mode: re-create missing boot entry (UEFI_BOOT_DESIGN.md §7)

| Step | Action | Expected | Actual | Date | Tester |
|---|---|---|---|---|---|
| 1 | With ARKON installed, delete the ARKON firmware entry with `efibootmgr -b XXXX -B` | "ARKON OS" entry gone | — | — | owner |
| 2 | Boot the installation medium; select Repair Mode | Medium boots; ARKON installation is detected | — | — | owner |
| 3 | Run Diagnose | Report shows FirmwareEntryMissing; recommends RecreateFirmwareEntry | — | — | owner |
| 4 | Run Recreate entry action | "ARKON OS" entry restored, pointing at shim on the ARKON ESP | — | — | owner |
| 5 | Reboot | ARKON boots normally | — | — | owner |

---

### HW-B-09 — Previous deployment recovery

Prerequisite: at least two ARKON deployments staged (requires a completed installation and at least one `bootc upgrade`).

| Step | Action | Expected | Actual | Date | Tester |
|---|---|---|---|---|---|
| 1 | From repair mode, run Diagnose | PreviousDeploymentAvailable(≥1) in report | — | — | owner |
| 2 | Run SelectPreviousDeployment | Previous deployment set as default | — | — | owner |
| 3 | Reboot | Older ARKON deployment boots | — | — | owner |

**Note:** The offline mechanism for switching the default deployment is unverified (UEFI_BOOT_DESIGN.md §7 open point; a repair-mode spike is needed).

---

---

## 2026-11-17 — arkon-installer state machine and screen tests

Host: Windows 11 / `cargo test`.  All tests use injected clock and no D-Bus, disk, or Tauri access.

| Suite | Tests | Passed | Failed |
|---|---|---|---|
| State transitions (Start → Complete) | 18 | 18 | 0 |
| Screen model and `can_proceed()` rules | 4 | 4 | 0 |
| Safety invariant: no writes before Executing | 2 | 2 | 0 |
| Failure, cleanup, cancel | 7 | 7 | 0 |
| Token, phrase, BitLocker confirmation | 3 | 3 | 0 |
| **arkon-installer total** | **34** | **34** | **0** |

### Test inventory

| Test | Scenario | Result |
|---|---|---|
| `start_to_preflight` | Start → Preflight transition | pass |
| `preflight_completes_to_scanned` | Preflight → Scanned | pass |
| `preflight_blocked_legacy_bios` | Preflight → Blocked (legacy BIOS) | pass |
| `rescan_voids_plan` | Rescan from Scanned voids existing plan | pass |
| `create_plan_transitions_to_planned` | Scanned → Planned | pass |
| `create_plan_requires_non_empty_id` | Empty plan ID rejected | pass |
| `set_configuration_transitions_to_configured` | Planned → Configured | pass |
| `set_configuration_validates_username` | Invalid username rejected | pass |
| `set_configuration_wrong_plan_id_rejected` | Plan ID mismatch rejected | pass |
| `confirm_correct_phrase_accepted` | Correct phrase + hash → Confirmed | pass |
| `confirm_wrong_phrase_rejected` | Wrong phrase → ConfirmationMismatch | pass |
| `confirm_hash_mismatch_rejected` | Wrong hash → PlanIdMismatch | pass |
| `confirm_missing_config_rejected` | Missing config → MissingConfiguration | pass |
| `confirm_non_destructive_no_phrase` | Non-destructive plan needs no phrase | pass |
| `confirm_bitlocker_ack_required` | BitLocker present → BitlockerAckRequired | pass |
| `execute_valid_token_starts_executing` | Valid token → Executing | pass |
| `execute_expired_token_returns_to_scanned` | Expired token → TokenExpired | pass |
| `execute_mismatched_token_rejected` | Wrong token → ConfirmationMismatch | pass |
| `advance_steps_through_x3` | X1 → X2 → X3 advance | pass |
| `cancel_before_x3_allowed` | Cancel during X1/X2 → ok | pass |
| `cancel_after_x3_refused` | Cancel after X3 → Busy | pass |
| `cancel_before_execution_allowed` | Cancel in Scanned/Planned/Configured/Confirmed → ok | pass |
| `fail_during_execution` | Fail carries DiskCondition + FailureClass | pass |
| `cleanup_success_returns_to_scanned` | Cleanup success → Scanned | pass |
| `cleanup_failure_returns_to_failed` | Cleanup failure → Failed | pass |
| `mark_verified_after_all_steps` | Executing → Verified after X12 | pass |
| `mark_complete_after_verified` | Verified → Complete | pass |
| `executing_state_has_writes_possible` | writes_possible() true only during Executing | pass |
| `no_state_before_executing_has_writes_possible` | writes_possible() false in all pre-Executing states | pass |
| `wrong_state_begin_preflight_twice` | WrongState error on duplicate transition | pass |
| `destructive_confirmation_phrase_match` | Matching phrase accepted | pass |
| `destructive_confirmation_phrase_mismatch` | Non-matching phrase rejected | pass |
| `welcome_screen_can_always_proceed` | Welcome screen can_proceed() = true | pass |
| `language_screen_requires_selection` | Language screen can_proceed() = false with no selection | pass |

---

## 2026-11-17 — arkon-firstboot integration tests

Host: Windows 11 / `cargo test`.  All tests use `MockProvider`; no host filesystem, D-Bus, or systemd access.

| Suite | Tests | Passed | Failed |
|---|---|---|---|
| Handoff data validation | 5 | 5 | 0 |
| Journal: restart-safe state | 6 | 6 | 0 |
| Full run and idempotency | 4 | 4 | 0 |
| NVIDIA GPU checks | 3 | 3 | 0 |
| Vulkan / audio / update / boot checks | 4 | 4 | 0 |
| Health report and diagnostics | 4 | 4 | 0 |
| Interrupted run resume | 2 | 2 | 0 |
| **arkon-firstboot total** | **28** | **28** | **0** |

### Test inventory

| Test | Scenario | Result |
|---|---|---|
| `handoff_validates_good_data` | Valid handoff data → ok | pass |
| `handoff_rejects_empty_username` | Empty username → error | pass |
| `handoff_rejects_username_with_spaces` | Username with spaces → error | pass |
| `handoff_rejects_username_too_long` | Username > 32 chars → error | pass |
| `handoff_rejects_empty_hostname` | Empty hostname → error | pass |
| `handoff_absent_returns_interactive_setup_flag` | No handoff → HandoffAbsent at CreateUser | pass |
| `fresh_journal_all_steps_pending` | New journal has all 11 steps Pending | pass |
| `journal_complete_step_recorded` | Completing a step records it as Complete | pass |
| `journal_complete_twice_returns_error` | Completing twice → AlreadyComplete | pass |
| `journal_failed_step_recorded` | Failed step recorded; failed_count() = 1 | pass |
| `journal_serialises_and_deserialises` | JSON round-trip preserves all step states | pass |
| `pending_steps_excludes_completed` | pending_steps() excludes Complete/Skipped/Warning | pass |
| `full_run_completes_all_steps` | Full run → all 11 steps Complete | pass |
| `full_run_produces_health_report` | Full run → health report with all_ok=true | pass |
| `create_user_skipped_when_user_exists` | Existing user → CreateUser skipped, not failed | pass |
| `journal_with_all_complete_returns_immediately` | finished=true journal → returns immediately | pass |
| `resumed_run_skips_already_complete_steps` | Pre-completed steps not re-executed | pass |
| `nvidia_step_skipped_when_no_nvidia_gpu` | NotPresent → Skipped | pass |
| `nvidia_step_complete_when_module_loaded` | ModuleLoaded → Complete | pass |
| `nvidia_module_failure_becomes_warning_not_abort` | ModuleFailed → Warning; run continues | pass |
| `vulkan_missing_records_warning_not_abort` | LoaderMissing → Warning; run completes | pass |
| `audio_not_running_records_warning` | NotRunning → Warning step status | pass |
| `update_config_missing_records_warning` | Missing → diagnostic in report | pass |
| `boot_deployment_mismatch_records_in_report` | Mismatch → all_ok=false | pass |
| `health_report_all_ok_when_everything_passes` | All Ok → all_ok=true, 0 errors | pass |
| `health_report_records_graphics_driver_missing` | Missing driver → error in graphics component | pass |
| `health_report_severity_sorted_errors_first` | Diagnostics sorted errors first | pass |
| `setup_step_failed_count_reflected_in_report` | 1 failed step → failed_count=1 in journal | pass |

---

### Open points for Phase 2 hardware verification

| ID | Statement | Test that resolves it |
|---|---|---|
| V-18 | Exact files, entry label and boot order created by bootc and bootupd | HW-B-03, HW-B-04 |
| V-19 | bootupd update behaviour with two ESPs on one disk | Requires same-disk dual-boot setup (U-25) |
| V-21 | Firmware boots from a second ESP on this hardware | Requires same-disk dual-boot setup (U-25) |
| V-22 | No Windows BitLocker recovery prompt after MOK enrolment | HW-B-07 extended with BitLocker enabled |
| V-29 | Current Fedora shim boots on firmware with only the 2011 Microsoft certificate | Check firmware version on ASUS FA506IH |
| V-31 | bootc signature enforcement works with scoped container policy | Requires Phase 1 image signing (P1-11) |
