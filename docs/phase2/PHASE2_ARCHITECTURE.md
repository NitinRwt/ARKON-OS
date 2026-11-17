# ARKON OS — Phase 2 Architecture: Bare-Metal Installer

Status: **DRAFT — design only.** Phase 2 is not open: the Phase 1 audit on record is "not approved" ([PHASE1_AUDIT.md](../../PHASE1_AUDIT.md)), and no image exists to install. This design rests on Phase 1 facts that have not been established and must be revalidated when they are (risk R-44).

Companion documents: [technology review](INSTALLER_TECHNOLOGY_REVIEW.md), [storage](STORAGE_ARCHITECTURE.md), [partitioning](PARTITIONING_DESIGN.md), [UEFI boot](UEFI_BOOT_DESIGN.md), [Secure Boot](SECURE_BOOT_DESIGN.md), [state machine](INSTALLATION_STATE_MACHINE.md), [API](INSTALLER_API_SPEC.md), [acceptance criteria](PHASE2_ACCEPTANCE_CRITERIA.md), [tasks](PHASE2_TASKS.md).

## 1. Goals and rules

Install ARKON OS from a USB medium onto a physical x86-64 UEFI machine, with Secure Boot on, including machines that already run Windows.

Rules the design enforces in code, not in documentation:

| # | Rule | Enforced by |
|---|---|---|
| S1 | Nothing is written to any disk before the user has confirmed a specific plan | State machine: no write step is reachable before `Confirmed` |
| S2 | A destructive plan needs a confirmation bound to that plan and that disk | The orchestrator checks a plan hash and a typed phrase (D-039) |
| S3 | The installer never shrinks, moves or reformats an existing partition, except by erasing a whole disk the user chose | Plan validation against the partitioner's dry-run (D-036, D-040) |
| S4 | The installer never writes to an EFI system partition it did not create | Planner always creates an ARKON-owned ESP (D-035) |
| S5 | Existing firmware boot entries are never deleted or edited | Before/after comparison; only the entry created in this run may be removed |
| S6 | The orchestrator exposes no method that runs a caller-supplied command | Fixed executables, typed arguments, no shell |
| S7 | Encrypted or unidentified partitions are treated as valuable and never selected automatically | Storage classification |
| S8 | Every failure leaves the user with a stated disk condition and a next step | Error model in the state machine document |

## 2. Technology choice

`systemd-repart` for partitioning, formatting and encryption; `bootc install to-filesystem` for deployment and bootloader; an ARKON orchestrator and front end around them (proposed decision D-034, replacing the Anaconda choice D-012). Reasons and alternatives are in the technology review. Anaconda driven by a generated kickstart remains the fallback for whole-disk installs only.

## 3. Installation modes and support level

| Mode | What happens | Level in this design |
|---|---|---|
| 1. Clean installation | The user picks one disk; it is erased and laid out by ARKON | **Core.** First deliverable |
| 3a. Dual boot, separate disks | Clean installation onto a disk that does not hold Windows. The Windows disk is not opened for writing | **Core.** Same code path as mode 1, plus detection and warnings |
| 3b. Dual boot, same disk | ARKON is added in unallocated space that already exists, with its own ESP. Existing partitions stay byte-identical | **Gated.** Ships only if V-18…V-22 pass on a disposable Windows install. Otherwise the installer explains how to use a second disk |
| 2. Manual partitioning | The user chooses the target region and options within fixed rules; not a free-form partition editor | **Constrained.** Stage 3 |
| 4. Existing Linux detection | Other Linux systems, including a previous ARKON, are detected and named; the user chooses replace, install beside (as 3b), or cancel | **Core for detection**, actions as per modes 1 and 3b |
| 5a. Boot repair | Re-create the ARKON firmware entry or reinstall ARKON's bootloader files on ARKON's own ESP | **Planned.** Stage 4 |
| 5b. Reinstall keeping home | Replace the OS, keep the `home` subvolume | **Where supported:** depends on V-25 and V-26. If they fail it is documented as back up, clean install, restore |

Never offered: shrinking Windows from the installer; installing onto a BitLocker or other encrypted volume; sharing an ESP; legacy BIOS installs; layouts spanning several disks; TPM unlock (D-030).

## 4. Components

```
 Installation medium (signed ISO, built by image-builder generic-iso)
 ┌───────────────────────────────────────────────────────────────────────┐
 │ Live environment = ARKON image + installer packages (Plasma session)  │
 │                                                                       │
 │  ┌──────────────────────┐   D-Bus, typed    ┌───────────────────────┐ │
 │  │ arkon-installer      │◄────────────────►│ arkon-installd        │ │
 │  │ Tauri front end      │  Installer1       │ orchestrator (root)   │ │
 │  │ unprivileged         │                   │ state machine, plans, │ │
 │  └──────────────────────┘                   │ validation, logging   │ │
 │  ┌──────────────────────┐                   └──────────┬────────────┘ │
 │  │ arkonctl install     │◄── same API ────────────────►│              │
 │  │ CLI, text fallback,  │                              │ fixed        │
 │  │ test harness         │                              │ argument     │
 │  └──────────────────────┘                              │ vectors      │
 │                                                        ▼              │
 │   libraries: arkon-hw (hardware, variant rule)   ┌───────────────┐    │
 │              arkon-storage (scan, classify,      │ systemd-repart│    │
 │              plan model, validation)             │ bootc         │    │
 │                                                  │ efibootmgr    │    │
 │   payloads: arkon and arkon-nvidia images in     │ mokutil       │    │
 │             the ISO's container storage          │ btrfs, mount  │    │
 │                                                  └───────────────┘    │
 └───────────────────────────────────────────────────────────────────────┘
                                   │ writes
                                   ▼
 Target disk: ARKON ESP · /boot · Btrfs (root, home) · handoff file
                                   │ first boot
                                   ▼
 Installed system: arkon-firstboot.service consumes the handoff, creates
 the user, applies locale, keyboard, time zone and host name, then removes it
```

| Component | Runs as | Responsibility | Does not do |
|---|---|---|---|
| `arkon-installer` (front end) | Live-session user | Wizard screens, plan display, confirmation entry, progress, error presentation | Any disk access, any privileged call other than the typed API |
| `arkon-installd` (orchestrator) | root, D-Bus activated, live environment only | Owns the state machine; scans; builds and validates plans; runs the fixed tools; verifies results; logs | Take commands, paths or image references from the caller |
| `arkon-storage` (library) | In the orchestrator | Read-only storage scan, classification, plan model, the safety validator | Write to a disk |
| `arkon-hw` (library, from D-024) | In the orchestrator | Hardware report, variant recommendation, Secure Boot and MOK state | — |
| `arkonctl install` | Live-session user | Drives the same API without a GUI; accepts a plan request file | Bypass confirmation |
| `arkon-firstboot.service` | root on the installed system, once | Applies the handoff | Network access |
| Executors | Children of the orchestrator | `repart-bootc` (primary). `anaconda-ks` (fallback; built only if needed) | — |

The orchestrator is not installed on the final system. The front end and orchestrator ship only in the live environment image.

## 5. Data flow

```
 probe ──► Environment ─┐
 scan  ──► StorageReport├─► PlanRequest ─► Plan ─► Confirmation ─► Execution ─► Result
 user choices ──────────┘      (typed)    (hash)   (phrase+hash)   (journal)   (verified)
```

1. **Probe.** `arkon-hw` reports firmware mode, Secure Boot and MOK state, CPU, GPUs and the recommended variant. Legacy BIOS boot stops here.
2. **Scan.** `arkon-storage` reads every disk without writing and classifies each partition and each detected operating system ([STORAGE_ARCHITECTURE.md](STORAGE_ARCHITECTURE.md)).
3. **Plan request.** The front end sends mode, target disk and options.
4. **Plan.** The orchestrator writes partition definitions, runs the partitioner in dry-run mode, converts its output into a `Plan`, and runs the safety validator. A plan lists every partition before and after, everything that will be lost, and warnings.
5. **Confirmation.** The user reads the plan and, for destructive plans, types a phrase derived from the target disk. The orchestrator accepts it only with the matching plan hash, and only if a fresh scan still matches the scan the plan was built from.
6. **Execution.** Steps run in a fixed order, each recorded in a journal with what it created ([INSTALLATION_STATE_MACHINE.md](INSTALLATION_STATE_MACHINE.md)).
7. **Verification.** The disk is rescanned and compared with the plan; firmware entries are compared with the snapshot taken before execution.
8. **Handoff.** User and locale settings are written to one file in the target's persistent state area for `arkon-firstboot`.

Secrets take a separate path: the encryption passphrase travels over a pipe to the orchestrator and is held in locked memory until the partitioner has used it; the user's password is hashed in the front end and only the hash is sent. Neither appears in logs.

## 6. User creation and first boot (D-037)

The installer does not edit files inside the deployed operating system. bootc gives no supported way to do so, and the deployment's path changes with every image. Instead:

- the orchestrator writes `handoff.json` (schema in the API specification) into the persistent state directory of the new installation, readable by root only;
- on first boot `arkon-firstboot.service` runs before the login screen, creates the administrator account with the supplied password hash, sets locale, keyboard, time zone and host name, deletes the file and disables itself;
- if no handoff exists, as after a manual `bootc install`, an interactive first-boot setup runs instead (KDE's initial-setup tool if Fedora ships it, V-28);
- if the handoff is present but invalid, the service leaves it in place, starts the interactive setup, and records the reason. The system is never left without a way to create a user.

After first login the session agent (D-025) shows the driver and enrolment state and, when online, installs the default Flatpak applications.

## 7. Logs and error handling

- One structured event log per run, plus the system journal of the live session. Every executed tool invocation is logged with its argument vector, exit status and duration. Secrets are never logged.
- On success the log is copied into the installed system. On failure the user can save it to a removable drive they choose, or view it.
- Every error has a code, a plain-language cause, the resulting disk condition (`unchanged`, `partially written`, `ARKON installed but not bootable`), and the recommended next step. The classes and recovery actions are in the state machine document.

## 8. Dependencies

| Depends on | For | State |
|---|---|---|
| Phase 1 images, signed and published | Payloads and the live environment base | **Does not exist** |
| `arkon-hw` library | Hardware report and variant rule | Not started; first Rust code of Phase 2 |
| Phase 1 results: boot chain facts (V-02), bootloader updates (V-15), signature enforcement (V-14), image and `/boot` sizes | Partition sizes, bootc flags | Unknown |
| Test hardware: a disk for clean installs and a **disposable Windows 11 install** for dual-boot tests | Every bare-metal criterion | Not available (U-01, U-25) |
| Owner decisions U-19…U-27 | Scope | Open |
| image-builder, systemd-repart, bootc, bootupd, efibootmgr, mokutil | Engine | Upstream, in Fedora |
| Tauri and WebKitGTK in the live session | Front end | Unverified (V-27); the CLI is the fallback |

## 9. Risks

Full entries are in [docs/RISK_REGISTER.md](../RISK_REGISTER.md).

| ID | Risk | Main mitigation |
|---|---|---|
| R-35 | Installation destroys another system's boot files on a shared ESP | Rule S4 |
| R-36 | Owning an installer exceeds the project's capacity | Staged delivery; fallback executor; stage 1 equals what Anaconda would have given |
| R-37 | bootupd or bootc touches more than the ARKON ESP when two exist on a disk | Mode 3b gated on V-18, V-19 |
| R-38 | Firmware loses or reorders boot entries and the user cannot reach ARKON or Windows | Fallback loader on ARKON's ESP; repair mode; documentation |
| R-39 | Windows asks for its BitLocker recovery key after installation | Detection, warning and acknowledgement before any write |
| R-40 | The partitioner changes an existing partition | Dry-run validation; post-execution comparison |
| R-41 | Handoff fails and no user exists, or the handoff leaks | Interactive fallback; root-only file; hash only; deleted after use |
| R-42 | Same-disk dual boot cannot be tested without a disposable Windows install | Mode stays gated until it is |
| R-43 | NVMe drive hidden by Intel RST/VMD firmware mode; no disk found | Detection with specific guidance |
| R-44 | This design was drawn before Phase 1 exists | Revalidation step at the real start of Phase 2 |
| R-45 | Microsoft's Secure Boot certificate transition leaves some firmware unable to verify the boot medium | Check on every test machine; documented firmware guidance |
| R-46 | The graphical front end does not render in the live session on some GPUs | CLI path through the same API |

## 10. Unresolved decisions

Recorded in [DECISIONS.md](../../DECISIONS.md).

| ID | Decision | Recommendation |
|---|---|---|
| U-19 | Installer engine: build option C, or stay with Anaconda | Option C, staged |
| U-20 | Same-disk dual boot in 1.0 | Only if its verification gate passes; separate-disk dual boot is the supported route |
| U-21 | Live Plasma session or installer-only kiosk session | Live session: users can check Wi-Fi, graphics and audio before installing |
| U-22 | Default firmware boot order after a dual-boot install | ARKON first, with the choice shown and reversible |
| U-23 | First-boot user creation: ARKON handoff service, or KDE's initial setup only | Handoff, with the KDE tool as fallback |
| U-24 | Recovery scope for 1.0 | Boot repair, and reinstall keeping home if V-25 and V-26 pass |
| U-25 | Hardware for a disposable Windows install | Needed before mode 3b or any BitLocker claim can be tested |
| U-26 | Languages and accessibility for 1.0 | English first; structure the front end for translation from the start |
| U-27 | A "restart into Windows" action in ARKON Center | Phase 4 candidate; firmware boot menu documented until then |

## Verification items

Added by Phase 2 design. None has been tested.

| ID | Must be shown | Decides |
|---|---|---|
| V-17 | `bootc install to-filesystem` deploys onto: ESP at `/boot/efi`, separate `/boot`, Btrfs subvolume as root, with and without LUKS; the exact flags and kernel arguments | Whether option C works at all |
| V-18 | Exactly what bootc and bootupd write during installation: ESP directories, fallback loader, firmware entry, its position in the boot order; and that no other ESP on any disk is touched | S4, S5, mode 3b |
| V-19 | Which ESP the automatic bootloader update writes to when a disk has two | Mode 3b |
| V-20 | Partition definitions exist that leave every existing partition unchanged while adding a second ESP; the dry-run output carries enough to prove it; a passphrase key works at the boot prompt; Btrfs subvolumes can be created | S3, the plan model |
| V-21 | Firmware boots correctly from the second of two ESPs on one disk, and what happens when the entry is lost | Mode 3b, repair mode |
| V-22 | Windows 11 boots unchanged with no recovery-key prompt after ARKON installation and after MOK enrolment; a Windows feature update leaves ARKON intact | Any dual-boot claim |
| V-23 | `generic-iso` can carry both payload images with shared layers; ISO size; the ISO boots with Secure Boot on | One ISO or two (replaces V-06) |
| V-24 | The new installation's persistent state directory can be written after deployment and before finalisation, at a stable path | D-037 |
| V-25 | How the `home` subvolume gets mounted on a bootc system without editing the deployment | Layout, mode 5b |
| V-26 | The OS subvolume can be replaced while `home` survives, with and without encryption | Mode 5b |
| V-27 | The Tauri front end renders in the live session on NVIDIA hardware without the proprietary driver, and on AMD | Front end or CLI by default |
| V-28 | KDE's initial-setup tool is packaged in Fedora and can run when no user exists | Fallback in section 6 |
| V-29 | Which Microsoft certificate signs Fedora's shim, and whether each test machine's firmware trusts it | R-45 |
| V-30 | Fallback only: Anaconda's `bootc` command installs from the ISO's local container storage and sets signature enforcement (replaces V-05) | Fallback executor |
| V-31 | Whether bootc's signature enforcement requires a restrictive default container policy, which would conflict with D-026 | D-026 |
