# ARKON OS — Test Results

Every entry states what was run, on what, against which artefact, and the result. Results that were not obtained are recorded as "not run" or "not verified", never left out.

## Summary

| Area | State |
|---|---|
| Unit tests | **160 passed, 0 failed** — arkon-hw (31) + arkon-storage (33) + arkon-boot (34) + arkon-installer (34) + arkon-firstboot (28) (2026-11-17, Windows host, `cargo test`) |
| Image builds | Not run — no image definition exists |
| Bare-metal tests | Not run — no image exists |
| Benchmarks | Not run |
| Phase 1 completion audit, 2026-11-16 | Ten repository and host inspections run; zero product tests possible. Phase 1 not approved. Details in [PHASE1_TEST_RESULTS.md](PHASE1_TEST_RESULTS.md) |
| Phase 2 automated configuration tests | 96 tests: arkon-boot 34 + arkon-installer 34 + arkon-firstboot 28 (2026-11-17). Physical hardware procedures documented in [PHASE2_TEST_RESULTS.md](PHASE2_TEST_RESULTS.md); not yet run. |

## 2026-11-17 — arkon-firstboot integration tests (crates/arkon-firstboot)

| Suite | Tests | Passed | Failed | Host | Date |
|---|---|---|---|---|---|
| Handoff data validation | 5 | 5 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Journal: restart-safe state | 6 | 6 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Full run and idempotency | 4 | 4 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| NVIDIA GPU checks | 3 | 3 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Vulkan / audio / update / boot checks | 4 | 4 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Health report and diagnostics | 4 | 4 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Interrupted run resume | 2 | 2 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| **arkon-firstboot total** | **28** | **28** | **0** | | |

All tests use `MockProvider`; no host filesystem, D-Bus, or systemd access during tests.

Key scenarios: handoff validates/rejects field constraints; absent handoff fails at CreateUser (fatal); fresh journal has all steps Pending; completed step not re-executed (AlreadyComplete error); journal round-trips through JSON; pending_steps excludes completed; full run completes all 11 steps; health report all_ok when everything passes; user skipped if already exists; interrupted run resumes from first pending step; finished journal returns immediately without re-running checks; NVIDIA not-present → Skipped; NVIDIA module loaded → Complete; NVIDIA module failed → Warning not abort; Vulkan missing → Warning; audio not running → Warning; update config missing → diagnostic in report; boot deployment mismatch → all_ok=false; graphics driver missing → error in report; diagnostics sorted errors-first.

## 2026-11-17 — arkon-installer state machine and screen tests (crates/arkon-installer)

| Suite | Tests | Passed | Failed | Host | Date |
|---|---|---|---|---|---|
| State transitions (Start → Complete) | 18 | 18 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Screen model and `can_proceed()` rules | 4 | 4 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Safety invariant: no writes before Executing | 2 | 2 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Failure, cleanup, cancel | 7 | 7 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Token, phrase, BitLocker confirmation | 3 | 3 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| **arkon-installer total** | **34** | **34** | **0** | | |

All tests are in-process; no D-Bus, no Tauri, no disk access. The session uses an injected clock (`with_clock`) so token-expiry tests are deterministic.

Key scenarios: Start → Preflight → Scanned → Planned → Configured → Confirmed → Executing → Verified → Complete; block from Preflight; rescan voids plan; plan ID mismatch rejected; set_configuration validates username/hostname; confirm rejects wrong hash, wrong phrase, missing BitLocker ack; execute rejects mismatched token; cancel allowed before X3, refused after; fail during execution carries disk condition; cleanup success returns to Scanned; cleanup failure stays Failed; `writes_possible()` is false in all pre-Executing states; screen `can_proceed()` gating for Welcome, Language, DestructiveConfirmation.

## 2026-11-17 — arkon-boot automated configuration tests (crates/arkon-boot)

| Suite | Tests | Passed | Failed | Host | Date |
|---|---|---|---|---|---|
| Firmware snapshot and diff | 7 | 7 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| ESP contents verification | 4 | 4 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Secure Boot state | 4 | 4 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| MOK enrolment planning | 4 | 4 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Bootloader validation | 3 | 3 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Repair-mode diagnostics | 5 | 5 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| Boot plan builder | 7 | 7 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| **arkon-boot total** | **34** | **34** | **0** | | |

All tests use `MockProvider`; no firmware, EFI variables, or ESP are accessed.  Detailed physical hardware test procedures are in [PHASE2_TEST_RESULTS.md](PHASE2_TEST_RESULTS.md).

Key scenarios covered: firmware-entry snapshot; diff violations (changed, removed, multiple new); Windows entry preservation; stale ARKON entry detection; ESP shim/grub/fallback verification; Secure Boot on/off/SetupMode; Secure Boot off is shown not refused; NVIDIA MOK enrolment queued/skipped/not-required; GRUB2+shim valid; systemd-boot flagged as unverified (not error); repair-mode diagnosis for missing entry/MOK/stale entry/previous deployment/healthy system; plan with GRUB2 valid; plan with Windows entry preserved; plan with NVIDIA + SB on queues MOK; plan with systemd-boot produces warning; plan with SB off produces warning; plan with stale entry produces warning.

## 2026-11-17 — arkon-storage unit and integration tests (crates/arkon-storage)

| Suite | Tests | Passed | Failed | Host | Date |
|---|---|---|---|---|---|
| `validator` unit tests | 8 | 8 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| `dryrun` unit tests | 2 | 2 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| `planner_integration` integration tests | 23 | 23 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| **arkon-storage total** | **33** | **33** | **0** | | |

Scenarios covered: empty disk, Windows-only disk, existing Linux disk, multiple NVMe drives, insufficient storage (disk and free-region), unsupported/supported sector sizes, encrypted (LUKS) partitions, install-medium / LVM / MBR blockers, dual-boot free-space preservation, reinstall-keep-home, stale scan ID, manual mode, missing disk ID, BitLocker acknowledgement.

## 2026-11-17 — arkon-hw unit tests (crates/arkon-hw)

| Suite | Tests | Passed | Failed | Host | Date |
|---|---|---|---|---|---|
| `cpu` | 6 | 6 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| `firmware` | 5 | 5 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| `gpu` | 6 | 6 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| `storage` | 8 | 8 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| `report` | 5 | 5 | 0 | Windows 11 / `cargo test` | 2026-11-17 |
| **Total** | **31** | **31** | **0** | | |

All tests use `MockProvider`; no host sysfs or /proc is read. Bare-metal probe not run — requires a Linux live session.

One test (`gpu::detects_intel_integrated`) initially failed due to an incorrect Arc discrete detection threshold; fixed before the final run.

## 2026-11-16 — Phase 0 environment inspection

Not a product test. Recorded because later decisions rest on it.

| Check | Method | Result |
|---|---|---|
| Repository contents | Directory listing | Empty; not a git repository |
| CPU | Windows hardware query | AMD Ryzen 5 4600H, 6 cores / 12 threads |
| GPU | Windows hardware query | NVIDIA GeForce GTX 1650 and AMD Radeon integrated graphics |
| Memory | Windows hardware query | 15.4 GB |
| Storage | Windows hardware query | One Samsung NVMe SSD, 477 GB |
| Firmware mode | Environment variable | UEFI |
| Secure Boot state | `Confirm-SecureBootUEFI` | **Not verified** — requires elevation |
| Container runtime | `docker version` | Docker Desktop installed; daemon not running |
| WSL | `wsl --list --verbose` | WSL2 present; only the `docker-desktop` distribution, stopped |
| Toolchain | Command lookup | git, rustc, cargo, node, npm, Python 3.11 present; podman, gh, pytest absent |

Not checked: whether the laptop has a free internal drive slot; the GPU's exact PCI device ID as Linux will report it; the firmware version; TPM presence.

## Phase 0 research — what was and was not confirmed

Confirmed from current sources: Fedora 45 release and versions; Plasma 6.8 being Wayland-only; bootc composefs backend status and its listed limitations; closure of the bootc systemd-boot tracker; archival of bootc-image-builder and the `generic-iso` type; NVIDIA branch split between 580 and the current branch; RPM Fusion's driver branches on Fedora 45; nvidia-container-toolkit being packaged in Fedora; Podman 6 removals; i686 packages remaining in Fedora 45.

## 2026-11-16 — Architecture review research

Confirmed from current sources during the review: the module-signing-only key usage OID and shim's handling of it (Ubuntu and GRUB developer documentation; not yet tested against Fedora's shim); NVIDIA's documentation that open kernel modules support runtime power-down on Ampere and newer only; Fedora's automatic bootloader update service and its extension to bootc systems; greenboot's Rust rewrite being packaged with bootc support; recurring Flatpak NVIDIA runtime mismatches after driver updates; CUDA 13's minimum GPU generation being Turing; `webkit2gtk-4.1` still being packaged in Fedora.

Not confirmed during the review: how bootc signature enforcement is set through Anaconda; greenboot's boot-counter details; whether the current NVIDIA branch still ships closed modules for Turing (the statement that both flavours continue comes from older NVIDIA material); TensorFlow's current GPU support.

The review itself was done by the documents' author. No independent reviewer has checked the architecture.

Not confirmed: items V-01…V-16 in PROJECT_STATE.md. Fedora's own bootc bare-metal documentation could not be read (the page returned an access-denied response to automated fetching), so statements about Anaconda's bootc installation behaviour rest on the osbuild documentation and prior knowledge.
