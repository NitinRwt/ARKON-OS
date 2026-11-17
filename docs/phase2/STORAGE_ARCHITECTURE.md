# ARKON OS — Storage Architecture (Installer)

Status: **DRAFT — design only.** Phase 2 is not open; see [PHASE2_ARCHITECTURE.md](PHASE2_ARCHITECTURE.md).

Scope: how the installer finds out what is on the machine's disks, how it classifies what it finds, and which targets it will and will not offer. What it then creates is in [PARTITIONING_DESIGN.md](PARTITIONING_DESIGN.md).

## 1. Principles

1. **Scanning never writes.** Every probe opens devices read-only. Filesystems are inspected through their signatures; the only mount the scanner performs is a read-only mount of FAT ESPs to list boot loader directories.
2. **Unknown means valuable.** Anything the scanner cannot identify, and anything encrypted, is treated as user data. It is never chosen automatically and never offered as free space.
3. **Free space means unallocated space in the partition table.** Empty-looking filesystems are not free space.
4. **Disks are named by stable identifiers.** Plans refer to `/dev/disk/by-id/…` paths and partition UUIDs, never to `/dev/sdX` names, which can change between scan and execution.
5. **The scan is repeated immediately before execution.** If it differs from the scan the plan was built on, the plan is void.

## 2. Scan pipeline

```
 block devices (udev) ──► disk records ──► partition records ──► classification
        │                     │                   │                    │
   transport, size,     table type, sector    type GUID, UUIDs,   role, encryption,
   model, removable,    sizes, free regions   filesystem signature detected OS,
   install medium?                                                 blockers
                                   firmware boot entries (read) ──► cross-reference
```

Sources, all read-only: udev properties, the kernel's block device attributes, partition tables, filesystem and container signatures, the firmware boot entry list, and the contents of ESPs.

## 3. Disk record

| Field | Meaning |
|---|---|
| `id` | Stable by-id path |
| `model`, `serial_tail` | Model string and last four characters of the serial, for the user to recognise the disk |
| `transport` | `nvme`, `sata`, `usb`, `mmc`, `other` |
| `size`, `logical_sector`, `physical_sector` | Capacity and sector sizes |
| `table` | `gpt`, `mbr`, `none` |
| `removable`, `is_install_medium` | The disk the installer booted from is never a target |
| `partitions[]`, `free_regions[]` | See below |
| `detected_os[]` | Operating systems found on this disk |
| `blockers[]`, `warnings[]` | Reasons the disk cannot be, or should not lightly be, a target |

## 4. Partition classification

| Class | Recognised by | Treatment |
|---|---|---|
| `esp` | ESP type GUID with a FAT filesystem | Never written. Contents listed read-only to detect operating systems |
| `msr` | Microsoft reserved type | Never touched |
| `windows-data` | Basic data type with NTFS | Never touched; never resized |
| `bitlocker` | BitLocker signature | Never touched; triggers the BitLocker acknowledgement |
| `windows-recovery` | Windows recovery type | Never touched |
| `linux-fs` | ext4, XFS, Btrfs and others | Named to the user; erased only as part of an explicit whole-disk or replace choice |
| `luks` | LUKS header | Opaque. Never opened or probed further without the user supplying a passphrase in a recovery mode |
| `lvm`, `md-raid`, `zfs`, `bcache` | Their signatures | Disk is reported; installing beside them is not offered |
| `swap` | Swap signature | Linux system indicator |
| `arkon` | ARKON's partition labels and marker (see partitioning design) | Enables the recovery modes |
| `unknown` | Anything else, including partitions with no recognisable signature | Treated as user data |

## 5. Operating system detection

| System | Evidence |
|---|---|
| Windows | `EFI/Microsoft/Boot/bootmgfw.efi` on an ESP; a firmware entry named "Windows Boot Manager"; MSR, NTFS or BitLocker partitions |
| ARKON | ARKON partition labels; the ARKON marker; a firmware entry whose loader sits on an ARKON ESP |
| Fedora and other bootc or ostree systems | Vendor directory on an ESP; an ostree repository on a Linux filesystem that can be identified from its superblock label or layout without mounting read-write |
| Other Linux | Vendor directories on an ESP; Linux filesystems; swap |
| Unidentified | A bootable ESP entry with no matching known system |

Detection is reported with its evidence so the user can judge it. It is advisory for naming and warnings; the safety rules do not depend on detection being right, because they forbid touching existing partitions regardless of what is in them.

## 6. Conditions that block or warn

| Condition | Result |
|---|---|
| Machine booted in legacy BIOS mode | Installation refused, with firmware guidance |
| No disk found, and a storage controller in RAID or Intel RST/VMD mode is present | Specific guidance: the drive is hidden by a firmware setting. Changing it can stop Windows booting, and the guidance says so |
| Disk smaller than the minimum (provisional: 64 GB) | Not offered |
| Disk is the installation medium | Not offered |
| MBR partition table | Offered for whole-disk erase only, which converts it to GPT. Free-space installation refused |
| LVM, software RAID, ZFS or firmware RAID members | Whole-disk erase only, behind the strongest warning; free-space installation refused |
| Disk holds Windows | Whole-disk erase needs the destructive confirmation naming Windows. Free-space mode available only if gated mode 3b is enabled |
| BitLocker volume anywhere on the machine | Acknowledgement required that the recovery key is saved, before any write to any disk |
| Windows hibernation or Fast Startup state | Not checked: the installer never writes to NTFS, so it does not matter for safety. Documentation still advises a full Windows shutdown |
| Target on USB transport | Allowed with a warning about performance and suspend |
| Laptop on battery below a threshold | Warning before execution |
| Unallocated region smaller than the minimum for free-space mode | Region not offered; the user is told how much is needed and that space must be freed from within Windows |
| Sector size other than 512 or 4096 logical | Refused |

## 7. Supported targets

| Target | Mode 1 clean | Mode 3b free space |
|---|---|---|
| NVMe drive | Yes | Yes, when the mode is enabled |
| SATA SSD or hard disk | Yes | Yes, when the mode is enabled |
| USB-attached drive | Yes, with a warning | No |
| eMMC, SD card | Not offered in 1.0 | No |
| Several disks in one layout | No | No |
| Hardware or firmware RAID volume | No | No |

Hard disks are accepted but ARKON is designed for solid-state storage; the installer says so when the target reports as rotational.

## 8. What the storage library exposes

The `arkon-storage` library has three parts with different privileges of use:

| Part | Function | Writes to disks |
|---|---|---|
| Scanner | Produces the `StorageReport` | Never |
| Planner | Turns a `PlanRequest` and a `StorageReport` into partition definitions and a `Plan` | Never |
| Validator | Checks a `Plan` against the safety rules, before execution and again against the rescanned disk afterwards | Never |

The only code path that writes is the executor in the orchestrator, and it writes only by invoking the partitioner and bootc with a validated plan.

## 9. Testing approach

- **Fixtures.** Classification and planning are tested against recorded scan data from real layouts: Windows 11 default, Windows with BitLocker, Windows plus Fedora, several ESPs, MBR disks, LUKS, LVM, empty disks, 4K-sector disks. Fixtures are collected from real machines by a read-only capture command.
- **Loopback disks.** The executor is tested in CI against disk images built to match the fixtures. These tests can prove that existing partitions remain byte-identical, by hashing them before and after.
- **Bare metal.** Loopback tests cannot show firmware behaviour. Every mode is repeated on physical hardware before it is declared supported.

## 10. Open points

| Point | Resolved by |
|---|---|
| Minimum disk and free-region sizes | Phase 1 image-size measurements |
| Whether the scanner should read SMART health and warn | Phase 2 design step; read-only, optional |
| How reliably an existing ostree system can be recognised without mounting | Fixture collection |
| Whether eMMC targets are worth supporting | After 1.0 |
