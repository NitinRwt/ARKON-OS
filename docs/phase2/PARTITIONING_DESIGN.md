# ARKON OS — Partitioning Design

Status: **DRAFT — design only.** Phase 2 is not open; see [PHASE2_ARCHITECTURE.md](PHASE2_ARCHITECTURE.md). Sizes are provisional until Phase 1 measures real images.

## 1. Target layout

One layout for every installation, encrypted or not, so that there is one thing to test.

```
 GPT disk
 ┌────────────┬────────────┬──────────────────────────────────────────────┐
 │ ARKON ESP  │ /boot      │ ARKON root                                   │
 │ FAT32      │ ext4       │ Btrfs (optionally inside LUKS2)              │
 │ 1 GiB      │ 2 GiB      │ rest of the target region                    │
 │ label      │ label      │ subvolumes:                                  │
 │ ARKON-ESP  │ ARKON-BOOT │   root   the OS deployments and system state │
 │            │            │   home   mounted at /var/home                │
 └────────────┴────────────┴──────────────────────────────────────────────┘
```

| Partition | Size | Why |
|---|---|---|
| ESP | 1 GiB | bootc's minimum is 512 MiB. The extra room allows for larger boot artefacts later without repartitioning |
| `/boot` | 2 GiB | Holds a kernel and initramfs for every retained deployment under the GRUB chain (D-003). Always separate, because bootc requires that with encryption and one layout is simpler than two |
| Root | Remainder | Btrfs with zstd compression (D-011) |

Subvolumes:

- `root` holds the operating system deployments and their persistent state (`/var` apart from home).
- `home` holds user data. It exists so that the OS can be replaced without touching it (mode 5b). How it gets mounted on a bootc system without editing the deployment is open (V-25); the candidates are a small generator shipped in the image, or a mount entry written by the first-boot service.

No swap partition. Fedora's compressed in-memory swap applies, and hibernation is unsupported (D-030).

Each ARKON partition carries a fixed partition label and the root filesystem carries a small marker file. Together these let a later run recognise an ARKON installation for the recovery modes without guessing.

## 2. Encryption

| Item | Design |
|---|---|
| What is encrypted | The root partition (both subvolumes). ESP and `/boot` are not, as the boot chain requires |
| Format | LUKS2, created by the partitioner |
| Unlock | Passphrase typed at boot. No TPM unlock in 1.0 (D-030) |
| Passphrase handling | Sent from the front end over a pipe, held in locked memory, handed to the partitioner as a key file on a memory-only filesystem, wiped after use. Never logged, never written to a disk |
| Strength check | Minimum length and a warning for weak passphrases; no silent rejection |
| Recovery key | A second, generated key is offered and shown once for the user to write down. Optional |
| Exposure accepted | Kernel and initramfs in `/boot` are not encrypted and not verified at boot beyond Secure Boot signatures on the kernel. Recorded in the security architecture as a 1.0 limit |

## 3. How partitioning is done

ARKON code does not call a partition editor or a filesystem maker. It writes declarative definition files for `systemd-repart` and invokes it with a fixed set of arguments.

```
 PlanRequest ──► definitions (one file per partition) ──► partitioner, dry run
                                                              │
                              Plan ◄── parse machine output ◄─┘
                               │
                         safety validator
                               │ ok
                 user confirmation (bound to plan hash)
                               │
                 partitioner, real run, same definitions
                               │
                 rescan ──► compare with Plan ──► continue or stop
```

Properties relied on, each to be proven in a spike (V-20):

- with the non-erasing mode, the partitioner only adds partitions in unallocated space;
- its dry run reports, per partition, whether it would be created, left unchanged or resized;
- it can create the LUKS container, the filesystems and the Btrfs subvolumes;
- the same definitions always produce the same result on the same disk.

The validator is the backstop if any of those is wrong. It never trusts the partitioner's intent; it checks the reported plan and then the actual result.

## 4. Mode by mode

### Mode 1 — clean installation (and 3a, separate-disk dual boot)

| Step | Detail |
|---|---|
| Target | One whole disk chosen by the user. Never preselected when more than one disk is eligible |
| Plan | Erase the partition table; create a new GPT with the target layout |
| Data lost | Everything on that disk. The plan lists each existing partition and each detected operating system by name |
| Other disks | Not opened for writing. Their ESPs and partitions are not part of the plan |
| Confirmation | Typed phrase derived from the disk (see state machine) |

For 3a the Windows disk is simply another disk. The installer shows it as "will not be changed".

### Mode 3b — dual boot on the same disk (gated)

| Step | Detail |
|---|---|
| Precondition | The disk is GPT and has one contiguous unallocated region of at least the minimum size. The user creates it beforehand with Windows' own Disk Management; the installer explains how and does not do it |
| Target | That unallocated region |
| Plan | Create an ARKON ESP, `/boot` and root inside the region. Every existing partition is listed as "unchanged" |
| Existing ESP | Left alone. ARKON gets its own (rule S4). Two ESPs on one disk are allowed by the UEFI specification; firmware and bootupd behaviour with them is what V-19 and V-21 must establish |
| Validator | Rejects the plan unless every pre-existing partition is reported unchanged in start, size, type and identifiers |
| After execution | Every pre-existing partition is re-read and compared. In test builds the comparison is a full content hash |
| Confirmation | Explicit, but not the destructive phrase, since nothing is erased |

If the gate does not pass, this mode is not shown, and the installer offers the second-disk route instead.

### Mode 2 — manual partitioning (constrained)

Not a general partition editor. The user can:

- choose which eligible unallocated region or which whole disk to use;
- choose encryption on or off;
- change the size of the ARKON root within the region, leaving the rest unallocated;
- choose to reuse an existing ARKON `home` subvolume (which is mode 5b).

The user cannot: assign arbitrary mount points, choose filesystems, place parts of ARKON on different disks, reuse a foreign partition, or share an ESP. Each of these is excluded because bootc or the boot chain cannot be shown to handle it, and an installer that accepts a layout it cannot boot is worse than one that refuses.

Users who need something else can partition by hand and run `bootc install to-filesystem` themselves; that path is documented as unsupported.

### Mode 4 — an existing Linux system is present

| Found | Choices offered |
|---|---|
| Another Linux on a disk the user selects | Erase that disk (mode 1, with the system named in the data-loss list); or use unallocated space beside it (mode 3b rules, same gate); or cancel |
| A previous ARKON installation | The above, plus the recovery modes |
| Linux on a disk the user did not select | Shown as "will not be changed" |

The installer does not offer to delete individual foreign partitions to make room. Removing a single partition is a decision with consequences the installer cannot see (another system's `/home`, a data volume); the user can do it deliberately with a partitioning tool and return.

### Mode 5a — boot repair

No partitioning. See [UEFI_BOOT_DESIGN.md](UEFI_BOOT_DESIGN.md).

### Mode 5b — reinstall keeping home (where supported)

| Step | Detail |
|---|---|
| Precondition | An ARKON layout is recognised by labels and marker; if encrypted, the user supplies the passphrase |
| Plan | Keep the partition table as it is. Recreate the filesystems on ESP and `/boot`. In the Btrfs volume, replace the `root` subvolume with a new one and keep `home` |
| Data lost | The operating system and system-wide state: installed system Flatpaks, system container storage, system settings. Listed in the plan |
| Data kept | Everything under home, including Steam libraries stored there and rootless containers |
| Confirmation | Destructive phrase, because system state is erased |
| Depends on | V-25, V-26. If either fails, this mode is replaced by documented back up and restore |

## 5. Validator rules

A plan is rejected unless all of these hold.

| # | Rule |
|---|---|
| P1 | The target disk is not the installation medium |
| P2 | Every partition the plan creates lies entirely inside the target region (the whole disk in erase mode; the chosen unallocated region otherwise) |
| P3 | In non-erase modes, every pre-existing partition on every disk is reported unchanged: same number, start, size, type and UUIDs |
| P4 | No operation refers to a partition on a disk other than the target disk |
| P5 | No ESP that existed before the run is formatted, mounted read-write or passed to bootc |
| P6 | The plan's data-loss list equals the set of existing partitions inside the target region. Nothing can be lost that is not listed |
| P7 | Sizes meet the minimums; alignment is to 1 MiB |
| P8 | In mode 5b, the partitions recreated are exactly the ones carrying ARKON labels on the recognised installation |
| P9 | The scan the plan was derived from is identical to a scan taken at confirmation time |

After execution the same rules are applied to the real disk. A mismatch stops the installation before deployment.

## 6. What can go wrong

| Failure | Disk condition | Handling |
|---|---|---|
| Dry run reports a change to an existing partition | Unchanged | Plan rejected; reported as an installer defect with the definitions attached |
| Partitioner fails during the real run in mode 1 | Target disk partially written; it was to be erased anyway | Retry is safe |
| Partitioner fails during the real run in mode 3b | New partitions may partly exist; existing ones verified unchanged | Offer to remove only the partitions created in this run, identified by the UUIDs recorded in the journal |
| Post-execution comparison finds an existing partition changed | Unknown | Stop. Nothing further is written. Show the saved copy of the original partition table and the documented manual restore procedure |
| Power loss during partitioning | As the two rows above, depending on mode | On next start the installer recognises an incomplete run from the absence of the marker and offers cleanup of ARKON-labelled partitions without a marker |

Before any write, the partition tables of the target disk are saved into the run's log so that the layout of a damaged table can be reconstructed by hand. The installer does not restore tables automatically; an automatic rewrite of a partition table on a disk in an unknown state is itself a destructive operation.

## 7. Open points

| Point | Resolved by |
|---|---|
| ESP, `/boot` and minimum root sizes | Phase 1 measurements |
| Mounting of the `home` subvolume | V-25 |
| Whether the `root`/`home` split should also separate system state for a "keep everything but the OS" reinstall | After V-26 |
| Exact partitioner options that guarantee non-growth of existing partitions | V-20 |
| Recovery key: on by default or opt-in | Phase 2 design step |
