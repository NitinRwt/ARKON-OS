# ARKON OS — UEFI Boot Design (Installer)

Status: **DRAFT — design only.** Phase 2 is not open; see [PHASE2_ARCHITECTURE.md](PHASE2_ARCHITECTURE.md). Several statements about bootupd and firmware behaviour are unverified and are marked.

## 1. Boot chain being installed

Unchanged from the system architecture (D-003): firmware → shim → GRUB2 → Fedora kernel, with bootloader files managed by bootupd. The installer does not place boot files itself. `bootc install to-filesystem` does it, through bootupd.

```
 Firmware boot manager
   ├─ entry "ARKON OS" ──► ARKON ESP: \EFI\fedora\shimx64.efi ──► grubx64.efi
   │                                                              │
   │                                              /boot (ext4): loader entries,
   │                                              kernel, initramfs per deployment
   │                                                              │
   │                                              root (Btrfs, maybe LUKS): ostree
   │
   ├─ entry "Windows Boot Manager" ──► Windows ESP (never written by ARKON)
   └─ fallback: \EFI\BOOT\BOOTX64.EFI on whichever ESP the firmware tries
```

The vendor directory is `EFI/fedora` because ARKON reuses Fedora's signed shim and GRUB, whose paths are built in (D-008, D-022). That is one more reason ARKON cannot share an ESP with a Fedora installation: both would claim the same directory.

## 2. ESP ownership (D-035)

**ARKON only writes to an ESP it created in this installation.**

| Situation | Behaviour |
|---|---|
| Clean install on a disk | New ESP on that disk |
| Windows on another disk | New ESP on the ARKON disk. The Windows disk and its ESP are not opened for writing |
| Same-disk dual boot (gated) | A second ESP inside the unallocated region. The existing ESP is untouched |
| Reinstall or repair of ARKON | The ESP that carries the ARKON label and belongs to the recognised installation |

Reason: bootc's installation empties the ESP it is given, and bootupd assumes it owns the vendor directory. Sharing would destroy the other system's boot files.

What must be verified before same-disk dual boot is enabled:

- bootc and bootupd write only to the ESP mounted for the new installation, and nothing to any other ESP on the same or another disk (V-18);
- the later automatic bootloader update (D-033) also writes only to ARKON's ESP when a disk has two (V-19). If bootupd updates every ESP on the disk, this mode is unsafe and stays off;
- firmware on the test machines boots from a second ESP through its entry (V-21).

## 3. Firmware boot entries

"Preserve existing EFI boot entries" is implemented as a measured property, not a hope.

| Step | Action |
|---|---|
| Before execution | Read and record every boot entry, the boot order, and the "next boot" value |
| During deployment | bootupd creates one entry for ARKON. Expected, from an upstream log, to be an `efibootmgr --create` pointing at shim on the target ESP; its default placement in the boot order is unverified (V-18) |
| After deployment | Read the entries again. Required: every entry that existed before still exists with identical contents; exactly one new entry exists and it points at the ARKON ESP |
| Boot order | Apply the user's choice (U-22): ARKON first, or the previous order with ARKON appended. This is the only firmware write the orchestrator performs itself, with one fixed command and a validated order list |
| On abort after the entry was created | Remove that one entry, identified by number, label and partition UUID recorded in the journal. No other entry is ever removed |

If the after-state fails the comparison, installation stops and reports exactly which entry changed, with the saved before-state for manual restoration.

Entries the installer never touches: Windows Boot Manager, other Linux systems, firmware-provided entries (network boot, recovery partitions, diagnostics), and removable-media entries.

Stale entries left by an earlier ARKON installation are detected (label matches, partition no longer exists) and shown; removal is offered as an explicit, separate action.

## 4. Resilience when firmware misbehaves

Firmware implementations lose entries after updates or battery loss, ignore boot order, or only honour the fallback path. Windows updates can also move Windows back to the front of the order.

| Mechanism | Purpose | State |
|---|---|---|
| Fallback loader on ARKON's ESP (`\EFI\BOOT\BOOTX64.EFI` is shim, with the helper that re-creates entries) | Booting "from the disk" in the firmware menu still reaches ARKON and restores its entry | Expected from Fedora's shim packaging; whether bootupd installs it in this path is V-18 |
| Repair mode on the installation medium (mode 5a) | Re-create the entry; reinstall bootloader files on ARKON's ESP | Designed below |
| Documentation of the firmware boot menu key | The user can always choose a system manually | Per-vendor table in user documentation |
| No automatic "fight back" | ARKON does not rewrite the boot order on every boot to stay first. Doing so makes dual-boot machines unpredictable and can loop with Windows doing the same | Decided |

On same-disk dual boot, the fallback path on the *first* ESP belongs to Windows. If the firmware loses ARKON's entry and only tries the first ESP, the machine boots Windows. That is safe, and repair mode recovers ARKON. It is a known limit of this mode.

## 5. Choosing an operating system at boot

ARKON's GRUB menu lists ARKON deployments only. bootupd uses a static GRUB configuration, and probing other operating systems from the boot loader is not part of it.

| Way to reach Windows | Status |
|---|---|
| Firmware boot menu key at power-on | Works everywhere; documented |
| Firmware boot order (U-22) | Chosen at install; changeable in firmware setup |
| "Restart into Windows" from the ARKON desktop (sets the firmware's next-boot value once) | Proposed for ARKON Center in Phase 4 (U-27). It would be one fixed, authenticated service method |
| Windows entry in ARKON's GRUB menu | Not planned. It would mean generating GRUB configuration on the client, which the bootupd model avoids |

Two side effects of dual booting that documentation must cover, neither caused by the installer: Windows and Linux store the hardware clock differently, and Windows' Fast Startup keeps its filesystems in a hibernated state.

## 6. Bootloader installation step

Performed by bootc, not by ARKON code. The orchestrator's responsibilities around it:

| Before | During | After |
|---|---|---|
| Mount the new root, `/boot` and the ARKON ESP under a private mount point; confirm by partition UUID that the mounted ESP is the one this run created | Run `bootc install to-filesystem` with a fixed argument list: source image from the ISO's local storage, target image reference from the image-shipped list, root and boot mount specifications, signature enforcement | Check the ESP contains the expected loader files; check `/boot` contains loader entries; compare firmware entries; apply boot order choice |

Kernel arguments come from the image (`/usr/lib/bootc/kargs.d`) and, for encryption and the root subvolume, from the orchestrator's fixed, validated set. The caller cannot supply kernel arguments.

## 7. Repair mode (5a)

Started from the installation medium when an ARKON installation is recognised.

| Action | What it does | Writes |
|---|---|---|
| Diagnose | Reads entries, ESPs, `/boot`, and reports what is missing or inconsistent | Nothing |
| Re-create boot entry | Adds an entry pointing at shim on the ARKON ESP | One firmware variable |
| Reinstall boot files | Runs bootupd's installation against the recognised ARKON ESP only | ARKON ESP |
| Boot the previous deployment once | Selects the older deployment as default | `/boot` loader entries — **mechanism unverified**; depends on how bootc exposes this offline |

Each action is a separate plan with its own confirmation. None touches any partition without an ARKON label.

## 8. Firmware conditions

| Condition | Handling |
|---|---|
| Legacy BIOS or CSM boot | Refused; guidance to boot the medium in UEFI mode |
| 32-bit UEFI on a 64-bit CPU | Unsupported |
| Firmware boot entry storage full or write-protected | Deployment succeeds but the entry cannot be created; reported, with the fallback path explained |
| Secure Boot | See [SECURE_BOOT_DESIGN.md](SECURE_BOOT_DESIGN.md) |
| Drive hidden behind Intel RST/VMD | Handled in storage detection |
| Removable or USB target | Entry is created; the fallback path makes the disk bootable on other machines too |

## 9. Bootloader updates after installation

Covered by D-033: Fedora's automatic bootloader update stays enabled, and an image that changes shim or GRUB is promoted only after a bare-metal boot. Installation-time consequence: the installer always deploys the bootloader that the image being installed carries, so a freshly installed system starts current.

## 10. Open points

| Point | Resolved by |
|---|---|
| Exact files, entry label and boot order placement produced by bootc and bootupd | V-18 |
| bootupd update behaviour with two ESPs | V-19 |
| Firmware behaviour with two ESPs, and entry loss, on real machines | V-21; only AMD laptop firmware is available today |
| Offline selection of the previous deployment | Repair-mode spike |
| Whether a later move to systemd-boot (D-003) changes this design | It would: different ESP contents and no bootupd. The ESP is sized with that in mind, but there is no in-place migration either way |
