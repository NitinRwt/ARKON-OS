# ARKON OS — Phase 2 Acceptance Criteria

Status: **DRAFT — design only.** Phase 2 is not open; see [PHASE2_ARCHITECTURE.md](PHASE2_ARCHITECTURE.md).

This replaces the Phase 2 section of [docs/ACCEPTANCE_CRITERIA.md](../ACCEPTANCE_CRITERIA.md) if the installer engine decision (U-19, D-034) is approved. The general gate rules there still apply: evidence in TEST_RESULTS.md with image digest, hardware, date and result; nothing untested is recorded as passed.

"R1" is the physical reference laptop. "Windows test disk" is a disposable Windows 11 installation on a disk that holds nothing of value, with BitLocker enabled for the tests that say so (U-25). No test in this document may be run against the owner's working Windows installation.

## Entry criteria

Phase 2 implementation does not start until all of these hold.

| # | Criterion | State on 2026-11-16 |
|---|---|---|
| E1 | Phase 1 has passed its audit, with evidence | **Not met.** Audit verdict on record: not approved |
| E2 | Signed `arkon` and `arkon-nvidia` images exist in a registry | Not met |
| E3 | Owner has decided U-19 (engine) and approved or rejected D-034…D-040 | Open |
| E4 | A clean-install test disk is available | Open (U-01) |
| E5 | This design has been revalidated against the facts Phase 1 established (boot chain, sizes, bootc flags, signature enforcement) | Not possible yet |

## Architecture approval (this document set)

| # | Criterion | State |
|---|---|---|
| A1 | Installer technologies evaluated with sources; unconfirmed claims marked | Done |
| A2 | All five requested installation modes designed, each with a stated support level | Done |
| A3 | Every requested component designed: front end, orchestrator, hardware and storage detection, partition planning, filesystem creation, bootloader installation, deployment, user creation, first boot, logs, error handling, recovery | Done |
| A4 | Every stated security requirement mapped to an enforcing mechanism | Done (rules S1–S8, P1–P9) |
| A5 | Diagrams, data flow, dependencies and risks included | Done |
| A6 | Unresolved decisions and unverified assumptions listed | Done (U-19…U-27, V-17…V-31) |
| A7 | Owner approval | **Open** |
| A8 | Independent review | Not done; the author reviewed their own work |

## Stage 1 — clean installation (modes 1 and 3a)

Must:

| # | Criterion |
|---|---|
| 1.1 | The ISO builds in CI from a tagged commit and is published with a checksum and a signature that verifies with the documented command |
| 1.2 | The ISO boots on R1 with Secure Boot on, without firmware changes |
| 1.3 | A clean installation to an empty disk on R1 completes and the installed system boots to the login screen with Secure Boot on |
| 1.4 | Installation completes with no network connection |
| 1.5 | The installer selects the NVIDIA variant on R1 without the user choosing. The rule lives only in `arkon-hw`, which has unit tests for every row of the GPU table |
| 1.6 | With more than one eligible disk, no disk is preselected |
| 1.7 | The installation medium is never offered as a target |
| 1.8 | No write occurs before confirmation: with a block-level write trace active, zero writes reach any disk in every state before `Executing` |
| 1.9 | A destructive plan cannot be executed without the correct phrase and plan hash (tested through the API, not only the front end) |
| 1.10 | A disk changed between planning and confirmation voids the plan |
| 1.11 | With a second disk present (mode 3a), that disk is byte-identical before and after installation, and every firmware entry that existed before still exists unchanged |
| 1.12 | The installed system updates from the `stable` stream without reconfiguration, and signature enforcement is recorded for its image source |
| 1.13 | The user account from the wizard exists at first boot; the handoff file is gone; no password or hash appears in any log |
| 1.14 | With the handoff removed before first boot, interactive setup appears and a user can be created |
| 1.15 | Booting the medium in legacy BIOS mode ends in a clear refusal |
| 1.16 | Every failure class F-PRE to F-DEPLOY has been provoked by fault injection and produced the documented state, disk condition and next step |
| 1.17 | The CLI completes the same installation from a request file, with no GUI |
| 1.18 | V-17, V-18, V-23, V-24, V-27, V-29 and V-31 are resolved |

Should: clean installation on a SATA SSD and on an NVMe drive (R1 can test NVMe only unless hardware is added); the front end rendering check on AMD-only and NVIDIA hardware.

## Stage 2 — encryption and enrolment

Must:

| # | Criterion |
|---|---|
| 2.1 | An encrypted installation completes; the system asks for the passphrase at boot and starts |
| 2.2 | The passphrase appears in no log, no file on any disk, and no process argument list (checked during a run) |
| 2.3 | The installer offers no TPM unlock |
| 2.4 | MOK enrolment queued by the installer leads to the enrolment screen at next boot on R1; after enrolling, the NVIDIA module loads |
| 2.5 | Declining enrolment yields a working desktop on the fallback graphics path and a visible prompt to enrol later |
| 2.6 | Reinstalling on a machine where the certificate is already enrolled does not ask again |
| 2.7 | The optional recovery key unlocks the volume |

## Stage 3 — same-disk dual boot and constrained manual mode (gated)

Gate: this stage is built only if V-19, V-20 and V-21 are resolved favourably. If not, criteria 3.x are replaced by one: the installer explains the second-disk route when Windows occupies the only disk.

Must, on the Windows test disk:

| # | Criterion |
|---|---|
| 3.1 | After installation into unallocated space, every pre-existing partition has an identical content hash, including the Windows ESP |
| 3.2 | Windows boots from its firmware entry and from the firmware boot menu, with no recovery-key prompt, with BitLocker enabled |
| 3.3 | Windows still boots with no recovery-key prompt after MOK enrolment |
| 3.4 | ARKON boots from its own entry; removing that entry and booting the disk through the firmware menu behaves as documented |
| 3.5 | After an ARKON update that changes shim or GRUB, the Windows ESP is still byte-identical |
| 3.6 | After a Windows feature update, ARKON's partitions are intact and repair mode restores its boot entry if Windows removed or reordered it |
| 3.7 | With a BitLocker volume present, execution is impossible without the recovery-key acknowledgement |
| 3.8 | The installer offers no option to shrink, move, format or mount read-write any pre-existing partition |
| 3.9 | An unallocated region below the minimum is not offered, and the message says how much is needed |
| 3.10 | An MBR disk, and a disk with LVM or RAID members, are refused for this mode |
| 3.11 | Power removed during step X3 and during step X6 (three trials each): all pre-existing partitions are identical afterwards, and the next start offers cleanup of the unfinished ARKON partitions only |
| 3.12 | Cleanup removes exactly the partitions and the firmware entry created by the run |
| 3.13 | Manual mode cannot produce a layout that fails to boot (every reachable combination is installed in loopback tests) |
| 3.14 | V-22 resolved |

## Stage 4 — detection and recovery (modes 4, 5a, 5b)

Must:

| # | Criterion |
|---|---|
| 4.1 | The scanner classifies each fixture layout correctly: Windows default, Windows with BitLocker, Windows plus Fedora, existing ARKON, LUKS, LVM, MBR, several ESPs, empty disk |
| 4.2 | An existing Linux installation on a selected disk is named in the data-loss list before a whole-disk erase |
| 4.3 | An unfinished ARKON installation is recognised and cleanup is offered |
| 4.4 | Repair mode re-creates a deleted ARKON firmware entry on R1 |
| 4.5 | Repair mode restores deleted boot files on the ARKON ESP on R1 and touches no other partition |
| 4.6 | Reinstall keeping home, with and without encryption: files in home are identical afterwards and the new system boots. Or: V-25 or V-26 failed, the mode is absent, and the backup-and-restore procedure is documented and has been followed once end to end |
| 4.7 | Repair and reinstall modes are offered only for installations recognised by label and marker |

## Phase 2 gate

Must:

| # | Criterion |
|---|---|
| G1 | Stage 1 and stage 2 criteria all pass |
| G2 | Stage 3 either passes entirely or is disabled with the documented alternative |
| G3 | Stage 4 criteria pass, with 4.6 in either of its two forms |
| G4 | Security review of the orchestrator: no generic execution primitive, no caller-controlled argument reaching a tool unvalidated, fuzzing of every method in every state shows no transition into `Executing` without a token |
| G5 | Rules S1–S8 and P1–P9 each have at least one automated test that fails when the rule is broken |
| G6 | User documentation exists for: verifying and writing the ISO; the pre-install checklist for machines with Windows; every installer screen; the MOK enrolment screen; every recovery procedure |
| G7 | A person other than the author installs ARKON on a machine from the documentation alone |
| G8 | D-003 (boot chain) re-decided, as already required at this gate |
| G9 | Hardware coverage is stated honestly: which storage types, firmware vendors and GPU classes were actually installed on. Classes not tested are listed as untested |

## What cannot be tested with the hardware available today

| Item | Why | Consequence |
|---|---|---|
| Any criterion needing the Windows test disk | No disposable Windows installation exists; the only Windows system is the owner's workstation | All of stage 3 and criterion 1.11 in its Windows form are blocked until U-25 is resolved |
| SATA targets | R1's only internal drive is NVMe | Untested unless a SATA drive is attached |
| Firmware other than one ASUS AMD laptop | One machine | Firmware-behaviour claims are limited to it |
| Intel RST/VMD detection | No Intel machine | Fixture-tested only |
| RTX-class GPUs, desktop NVIDIA without an integrated GPU | None available | Variant selection is unit-tested from recorded hardware data; live-session rendering on such machines is untested |
