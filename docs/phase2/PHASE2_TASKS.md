# ARKON OS — Phase 2 Tasks

Status: **DRAFT — planning only.** No task here may start until the entry criteria in [PHASE2_ACCEPTANCE_CRITERIA.md](PHASE2_ACCEPTANCE_CRITERIA.md) are met. Today the first of them is not: Phase 1 is not approved and no image exists.

Estimates are engineer-weeks for one full-time engineer and are unvalidated.

## Gate before any work

| ID | Task | Owner |
|---|---|---|
| P2-00a | Complete Phase 1 and pass its re-audit | Engineering |
| P2-00b | Decide U-19 (installer engine); approve or reject D-034…D-040 | Project owner |
| P2-00c | Provide a clean-install test disk (U-01) and decide on a disposable Windows test installation (U-25) | Project owner |
| P2-00d | Revalidate this design against what Phase 1 established; correct the documents first | Engineering |

## Stage 0 — spikes (about 2 weeks)

Throwaway experiments on loopback disk images and then on the test disk. Each ends with a written result in TEST_RESULTS.md and resolves the named verification items. No product code is kept from a spike.

| ID | Spike | Resolves | Decides |
|---|---|---|---|
| P2-S1 | Lay out a disk with the partitioner from definition files, then deploy with `bootc install to-filesystem`: separate `/boot`, Btrfs subvolume root, without and with LUKS. Record the exact options and kernel arguments | V-17, V-20 (part) | Whether the primary executor is viable. If not, fall back to Anaconda for stage 1 (V-30) and stop stage 3 planning |
| P2-S2 | Observe everything bootc and bootupd write: ESP contents, firmware entry, boot order; repeat with a second ESP present on the same disk and on another disk | V-18 | Rules S4 and S5 as designed, or redesign |
| P2-S3 | With two ESPs on one disk, apply a bootloader update and record which ESP changes | V-19 | Stage 3 on or off |
| P2-S4 | Definitions that add partitions beside existing ones and leave the existing ones unchanged; dry-run output fields; passphrase key at the boot prompt | V-20 | Plan model |
| P2-S5 | Write to the new installation's persistent state directory after deployment; mount of the `home` subvolume without editing the deployment | V-24, V-25 | Handoff location; layout |
| P2-S6 | Build a `generic-iso` with both payloads; measure size; boot it on R1 with Secure Boot on; check the shim's signing certificate against R1's firmware | V-23, V-29 | One ISO or two |
| P2-S7 | Run a minimal Tauri window in the live session on R1 | V-27 | Graphical front end by default, or CLI first |
| P2-S8 | Signature enforcement option with the scoped policy | V-31 | D-026 construction |

## Stage 1 — clean installation (about 4–5 weeks)

| ID | Task | Criteria |
|---|---|---|
| P2-10 | `arkon-hw` library and `arkon-hwprobe`: hardware report, GPU table, variant rule, unit tests from recorded hardware data | 1.5 |
| P2-11 | `arkon-storage` scanner and classification, with the fixture capture command and first fixtures | 1.6, 1.7 |
| P2-12 | D-Bus interface XML for `Installer1`; generated types; conformance test skeleton | — |
| P2-13 | Orchestrator skeleton: state machine, preflight, journal, run log | 1.8 |
| P2-14 | Planner and validator for `erase_disk`; dry-run parsing; plan hashing | 1.9, 1.10 |
| P2-15 | Confirmation tokens | 1.9 |
| P2-16 | Executor: partition, verify, mount, deploy, read back | 1.3, 1.12 |
| P2-17 | Firmware entry snapshot, comparison and boot order policy | 1.11 |
| P2-18 | Handoff writer; `arkon-firstboot.service` in the OS image; interactive fallback | 1.13, 1.14 |
| P2-19 | CLI client | 1.17 |
| P2-20 | Front end: environment, disk choice, review, confirmation, configuration, progress, result, failure screens | 1.3 |
| P2-21 | Live environment image and ISO build in CI; payload list; ISO signing | 1.1, 1.2, 1.4 |
| P2-22 | Loopback test harness in CI: installs, hashes untouched disks, fault injection | 1.8, 1.11, 1.16 |
| P2-23 | Bare-metal runs on R1; results recorded | 1.2, 1.3, 1.15 |
| P2-24 | User documentation: verify and write the ISO, pre-install checklist, screens | G6 |

## Stage 2 — encryption and enrolment (about 2 weeks)

| ID | Task | Criteria |
|---|---|---|
| P2-30 | Passphrase path: descriptor passing, locked memory, key file on memory-only storage, wiping | 2.1, 2.2 |
| P2-31 | Encrypted layout in planner, executor and kernel arguments; optional recovery key | 2.1, 2.7 |
| P2-32 | MOK queueing from the live environment; final-screen guidance | 2.4, 2.5, 2.6 |
| P2-33 | Secret-leak tests: logs, files, process arguments | 2.2 |

## Stage 3 — same-disk dual boot and manual mode (about 3–4 weeks; only if P2-S3 and P2-S4 allow)

| ID | Task | Criteria |
|---|---|---|
| P2-40 | Planner and validator for `free_space`; rules P2, P3, P5, P6 | 3.1, 3.8 |
| P2-41 | Second-ESP creation and its checks | 3.1, 3.4 |
| P2-42 | BitLocker detection and acknowledgement | 3.7 |
| P2-43 | Cleanup of a run's own partitions and entry; unfinished-installation detection | 3.11, 3.12 |
| P2-44 | Constrained manual mode | 3.13 |
| P2-45 | Loopback tests with content hashes for every fixture | 3.1, 3.10 |
| P2-46 | Bare-metal tests on the Windows test disk, including BitLocker, a Windows feature update and power-loss trials | 3.2–3.6, 3.11 |
| P2-47 | User documentation for dual boot: freeing space in Windows, recovery key, boot menu, clock and Fast Startup notes | G6 |

If the gate fails: replace this stage with one task, the second-disk guidance screen and its documentation.

## Stage 4 — detection and recovery (about 2–3 weeks)

| ID | Task | Criteria |
|---|---|---|
| P2-50 | Remaining fixtures and classifications; data-loss naming of detected systems | 4.1, 4.2 |
| P2-51 | Repair mode: diagnose, re-create entry, reinstall boot files | 4.4, 4.5 |
| P2-52 | Spike then implementation: replace the OS subvolume and keep `home` | V-26, 4.6 |
| P2-53 | Recognition by label and marker; guard on recovery modes | 4.3, 4.7 |
| P2-54 | Recovery documentation | G6 |

## Closing (about 1 week)

| ID | Task | Criteria |
|---|---|---|
| P2-60 | Security review and API fuzzing | G4, G5 |
| P2-61 | Installation by a second person from documentation | G7 |
| P2-62 | Re-decide D-003; update DECISIONS.md | G8 |
| P2-63 | Hardware coverage statement; phase review; gate | G9 |

## Order and dependencies

```
 P2-00 gate ─► Stage 0 spikes ─┬─► Stage 1 ─► Stage 2 ─┬─► Stage 4 ─► Closing
                               │                        │
                               └─ S3, S4 result ────────┴─► Stage 3 (or its replacement)
```

- Stage 1 is the minimum useful installer and equals what an Anaconda-based installer would have delivered.
- Stage 3 is the only stage that needs the Windows test disk. It can be dropped without affecting the others.
- Stages 3 and 4 can be swapped if the Windows test disk arrives late.

## Total

About 14–17 engineer-weeks with every stage, about 11–13 without stage 3. This is larger than the four to six weeks the Phase 0 roadmap allowed for an Anaconda-based installer, and the roadmap is updated only if U-19 is approved.
