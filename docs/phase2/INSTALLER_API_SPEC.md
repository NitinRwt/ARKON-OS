# ARKON OS — Installer API Specification

Status: **DRAFT — design only.** Phase 2 is not open; see [PHASE2_ARCHITECTURE.md](PHASE2_ARCHITECTURE.md). Shapes are specified here; the authoritative definition will be the D-Bus introspection XML, from which Rust and TypeScript types are generated. The bus name prefix `org.arkonos` is a placeholder until U-07 is answered.

## 1. Interfaces covered

| Interface | Between | Form |
|---|---|---|
| `org.arkonos.Installer1` | Front end or CLI ↔ orchestrator | D-Bus, system bus of the live environment |
| Hardware report | `arkon-hw` → orchestrator, front end | Rust type; JSON from `arkon-hwprobe` |
| Plan request file | CLI and test harness → orchestrator | JSON, same schema as `PlanRequest` |
| First-boot handoff | Orchestrator → `arkon-firstboot.service` | One JSON file |
| Payload list | ISO build → orchestrator | JSON inside the live image |
| Run log | Orchestrator → user, support | JSON lines |
| Executor contract | Orchestrator → partitioner, bootc and other tools | Fixed argument vectors |

## 2. Rules for the whole API

1. No method accepts a command, a shell fragment, a kernel argument, a mount option or an image reference from the caller.
2. Disks and partitions are referred to only by identifiers that came from the most recent `StorageReport`.
3. Every string is validated against a closed set or a strict pattern before use.
4. Secrets cross the bus as file descriptors, never as message arguments.
5. Methods are valid only in the states listed; otherwise they fail with `WrongState`.
6. The interface is versioned by name. An incompatible change creates `Installer2`.

## 3. `org.arkonos.Installer1`

### Methods

| Method | Valid in | Returns | Effect |
|---|---|---|---|
| `GetState()` | Any | `State` | None |
| `GetEnvironment()` | After Preflight | `Environment` | None |
| `ScanStorage()` | Blocked, Scanned, Planned, Configured, Failed | `StorageReport` | Rescan. Voids any plan. → Scanned |
| `CreatePlan(PlanRequest)` | Scanned, Planned, Configured | `Plan` | Dry run and validation only. → Planned |
| `SetSystemConfiguration(plan_id, SystemConfiguration)` | Planned, Configured | — | Stores user and locale settings |
| `ProvideEncryptionPassphrase(plan_id, fd)` | Planned, Configured, when the plan has encryption | — | Reads the passphrase from the descriptor into locked memory |
| `ProvideUnlockPassphrase(disk_id, fd)` | Scanned, for recovery modes | `UnlockResult` | Verifies a passphrase against an ARKON encrypted volume without mounting read-write |
| `Confirm(plan_id, plan_hash, Confirmation)` | Configured | `token` | Rescans; compares; checks phrase and acknowledgements. → Confirmed |
| `Execute(plan_id, token)` | Confirmed | — | Starts execution. → Executing |
| `Cancel()` | Any state before step X3 | — | → Scanned |
| `GetFailure()` | Failed | `Failure` | None |
| `CleanUp(CleanupRequest)` | Failed, or Scanned with an incomplete installation detected | `CleanupResult` | Removes only this run's items, or unmarked ARKON partitions |
| `QueueMokEnrollment()` | Verified, Complete | `one_time_password` | Queues the ARKON certificate |
| `ExportLog()` | Any | `fd` | Read-only descriptor to the run log |
| `Reboot()` | Complete, Failed, Blocked | — | Restarts through the system manager |

### Signals

| Signal | Payload |
|---|---|
| `StateChanged` | `state`, `step?` |
| `Progress` | `step`, `fraction`, `message_code` |
| `LogEvent` | One log record |

### Errors

`org.arkonos.Installer1.Error.` followed by: `WrongState`, `UnknownIdentifier`, `InvalidArgument`, `PlanRejected` (carries the rule number), `StalePlan`, `ConfirmationMismatch`, `TokenExpired`, `MissingConfiguration`, `MissingSecret`, `Busy`.

Execution failures are not D-Bus errors. They are the `Failed` state with a `Failure` record, because they outlive the call that started them.

## 4. Types

Field names are final in intent; encodings are fixed when the XML is written.

```
State
  state            start | preflight | blocked | scanned | planned | configured
                   | confirmed | executing | verified | complete | failed | cleaning_up
  step             X1..X12, when executing
  plan_id          present from planned onward

Environment
  installer_version
  firmware         { mode: uefi | bios, bits }
  secure_boot      { enabled, setup_mode, arkon_cert_enrolled }
  power            { on_ac, battery_percent? }
  memory_mib
  hardware         Capabilities            (from arkon-hw; includes recommended_variant)
  payloads[]       { variant, version, digest, size_bytes, driver_version? }
  blockers[]       { code, detail }
  warnings[]       { code, detail }

StorageReport
  scan_id          opaque; changes when anything on any disk changes
  disks[]          Disk
  firmware_boot    { entries[] { number, label, part_uuid?, loader_path?, active },
                     order[], current, next? }
  detected_os[]    { kind: windows | arkon | fedora | linux | unknown,
                     name, disk_id, evidence[] }
  bitlocker_present  bool

Disk
  id               stable by-id path
  model, serial_tail, transport, size_bytes, logical_sector, physical_sector
  table            gpt | mbr | none
  removable, is_install_medium, rotational
  partitions[]     Partition
  free_regions[]   { region_id, start, size_bytes }
  eligible_modes[] subset of the PlanRequest modes
  blockers[], warnings[]

Partition
  part_uuid, number, start, size_bytes, type_guid, type_name
  label?, fs_type?, fs_uuid?, fs_label?
  class            esp | msr | windows-data | bitlocker | windows-recovery | linux-fs
                   | luks | lvm | md-raid | zfs | swap | arkon | unknown
  arkon_role?      esp | boot | root        (only for class arkon)

PlanRequest
  mode             erase_disk | free_space | manual | reinstall_keep_home
                   | repair_boot_entry | repair_boot_files
  scan_id          must equal the current scan
  target_disk      Disk.id
  region_id?       for free_space and manual
  root_size_bytes? manual only; within the region
  encryption       bool
  variant?         one of Environment.payloads[].variant; default is the recommendation
  boot_order       arkon_first | keep_current

Plan
  plan_id, plan_hash, scan_id, mode, target_disk, variant
  operations[]     { action: create | format | replace_subvolume | keep | erase_table
                             | firmware_entry_create | boot_files_install,
                     partition?: reference, before?: Partition, after?: PlannedPartition }
  unchanged[]      every existing partition on every disk that the plan leaves alone
  data_loss[]      { part_uuid, description, detected_os? }
  destructive      bool
  warnings[]       { code, detail }
  confirmation     { phrase_required, phrase_hint, bitlocker_ack_required }
  definitions      the partitioner definition files, for the log and for audit

SystemConfiguration
  user             { name, full_name, password_hash, administrator: true }
  hostname, locale, keymap, timezone

Confirmation
  phrase?          typed by the user
  bitlocker_ack?   bool
  mok_enroll       accept | decline | not_applicable

Failure
  class            F-PRE … F-POWER       (state machine document)
  step, code, detail
  disk_conditions[]   { disk_id, condition: unchanged | incomplete_target
                        | arkon_not_bootable | arkon_installed | unexpected_change }
  other_os_verified   bool: true only if the post-execution comparison ran and passed
  cleanup_available   bool
  next_steps[]        message codes
```

Validation patterns: user name per the system's account-name rules and not a reserved name; host name per RFC 1123 labels; locale, keymap and time zone must be members of the lists the live image provides; `password_hash` must be a hash in the scheme the target system accepts, never plain text.

## 5. First-boot handoff

Written by the orchestrator in step X8; consumed once by `arkon-firstboot.service`.

| Property | Value |
|---|---|
| Location | A fixed file in the new installation's persistent state directory (path fixed after V-24) |
| Ownership and mode | root, readable by root only |
| Lifetime | Deleted after successful use |
| Contents | Schema version, `SystemConfiguration`, installed variant, installer version, MOK enrolment choice, timestamp |
| Never contains | Plain-text passwords, the encryption passphrase, disk serial numbers |
| Unknown schema version or failed validation | File kept; interactive setup runs; reason recorded |

## 6. Payload list

A JSON file inside the live image, produced when the ISO is built: for each payload, the variant name, version, image digest, target image reference for updates, and driver version. The orchestrator installs only payloads on this list and refuses one whose digest does not match. The target image reference used for the installed system comes from here, never from the caller.

## 7. Run log

One JSON object per line.

| Field | Content |
|---|---|
| `ts`, `seq` | Time and sequence number |
| `level` | info, warning, error |
| `state`, `step` | Where the machine was |
| `event` | A code from a closed list |
| `data` | Structured detail: scan summaries, plans, tool invocations with argument vectors, exit status, duration |

Excluded by construction: passphrases, password hashes, the MOK one-time password. Included, because diagnosis needs them: disk models, serial tails, partition UUIDs. The export screen says what the log contains before the user shares it.

## 8. Executor contract

The orchestrator may start exactly these programs, each with an argument vector assembled from typed values. Adding a program or an argument form is an interface change that needs review.

| Program | Purpose | Notes |
|---|---|---|
| `systemd-repart` | Dry run and real run against the target disk with the definitions directory of this plan | Target given as a by-id path that has been matched to the plan |
| `cryptsetup` | Unlock and lock the volume created or recognised in this run | Key over a descriptor |
| `btrfs` | Create or replace the two subvolumes | Only on the volume created or recognised in this run |
| `mount`, `umount` | Private mount namespace under a fixed directory | Fixed option sets |
| `bootc` | `install to-filesystem` with fixed options; source from the payload list | No caller-supplied arguments |
| `bootupctl` | Repair mode only, against the recognised ARKON ESP | — |
| `efibootmgr` | Read entries; set boot order; create or remove ARKON's own entry | Order list validated against the snapshot |
| `mokutil` | Test and queue the ARKON certificate | Password on standard input |
| `sync`, `fstrim` | Finalisation | — |

Every invocation is logged before it starts. There is no generic "run" primitive in the orchestrator's code, and no program is started through a shell.

## 9. Fallback executor

If stage 1's spike shows the primary executor is not workable (U-19), the same `Plan` is translated into a kickstart for Anaconda's `bootc` command, for `erase_disk` only. The API above does not change; `eligible_modes` simply never lists the other modes. This is the reason the plan model is kept independent of the partitioner.

## 10. Compatibility and testing

- The CLI accepts a `PlanRequest` file and a `SystemConfiguration` file and drives the same methods, which makes every mode scriptable on test hardware and in loopback tests.
- A conformance suite exercises every method in every state and checks that the result is either the documented effect or `WrongState`.
- Generated types are checked for drift against the XML in CI, as for the other ARKON interfaces.

## 11. Open points

| Point | Resolved by |
|---|---|
| Handoff file path | V-24 |
| Whether the orchestrator needs polkit at all in a live environment where the session user is an administrator | Phase 2 design step; the confirmation token is the control that matters |
| Translation: message codes are stable identifiers so that front ends can localise; first set is English (U-26) | Stage 1 |
| Whether `QueueMokEnrollment` reuses the installed system's `SecureBoot1` implementation as a shared crate | Stage 2 |
