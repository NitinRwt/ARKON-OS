# ARKON OS — Secure Boot Design (Installer)

Status: **DRAFT — design only.** Phase 2 is not open; see [PHASE2_ARCHITECTURE.md](PHASE2_ARCHITECTURE.md). This document applies the project's security architecture to the installation medium and the installation process; it does not change the trust model.

## 1. Position

- Installation works with Secure Boot **on**, without the user changing a firmware setting, on machines whose firmware trusts Microsoft's third-party UEFI certificate.
- The installer never asks the user to turn Secure Boot off and never describes that as a fix.
- Installation also works with Secure Boot off or absent; the state is shown, not judged.

## 2. What is signed, by whom

| Artefact | Signed by | Verified by | When |
|---|---|---|---|
| shim on the ISO and on the installed ESP | Microsoft (third-party UEFI CA), shipped by Fedora | Firmware | Every boot |
| GRUB, kernel | Fedora | shim, then GRUB | Every boot |
| NVIDIA kernel modules in the installed system | ARKON module key, module-signing only (D-029) | Kernel, after MOK enrolment | Module load |
| The ISO file | ARKON image key (detached signature) plus published checksums | The user, before writing the USB stick | Once; documented step |
| OS images in the registry | ARKON image key | Container tooling under the scoped policy (D-026) | Every update |
| Payload images inside the ISO | Not individually verifiable offline; covered by the ISO signature and by a digest list inside the live image | Orchestrator compares each payload's digest with the list before deployment | Every installation |

ARKON signs no boot loader and no kernel in 1.0. Everything the firmware and shim verify is Fedora's. That keeps the installation medium bootable without any key enrolment.

## 3. Boot of the installation medium

```
 firmware (Secure Boot on) ─► shim on the ISO ─► GRUB ─► Fedora kernel ─► live ARKON image
                                                                        (Mesa or generic
                                                                         graphics only)
```

- The live environment never needs an ARKON-signed module. On NVIDIA machines it runs on the open nouveau driver or the firmware framebuffer. This is deliberate: the ARKON certificate is not enrolled yet, so the NVIDIA module could not load.
- Kernel lockdown is active in the live environment. The orchestrator uses nothing that lockdown blocks.

## 4. Firmware conditions and what the installer does

| Condition | Detection | Behaviour |
|---|---|---|
| Secure Boot on, standard keys | Firmware variables | Normal path |
| Secure Boot off | Firmware variables | Normal path. Final screen notes that the installed system also supports Secure Boot on |
| Secure Boot on, Microsoft third-party certificate not trusted (some "secured-core" laptops) | The medium does not boot at all | Cannot be handled in the installer. Documentation names the firmware option that allows third-party certificates; this is not disabling Secure Boot |
| Setup mode or custom keys | Firmware variables | Shown as an advanced configuration; installation proceeds if the medium booted |
| Firmware lacks the newer Microsoft certificate | Depends on which certificate signed Fedora's shim | **Unverified (V-29).** Microsoft's 2011 third-party certificate reached its expiry in 2026 and a 2023 certificate succeeds it. Whether current Fedora media boot on firmware that carries only the older one must be checked on each test machine. If it fails, the remedy is a firmware update, not disabling Secure Boot |
| Revocation level on the machine newer than the medium's shim | The medium is refused by firmware | Documentation: download a current ISO. ARKON rebuilds the ISO whenever the image's shim changes |

## 5. MOK enrolment during installation (NVIDIA variant)

Enrolment is queued from the live environment so that the first boot of the installed system already has a working NVIDIA driver.

| Step | Detail |
|---|---|
| When | After successful deployment, only if the NVIDIA variant was installed and Secure Boot is on |
| Skip if | The ARKON certificate is already enrolled (reinstall), or Secure Boot is off |
| How | The orchestrator queues the certificate from the payload image with `mokutil`, generating a one-time password and passing it on standard input |
| What the user sees | The final screen shows the password, a picture of the enrolment screen that will appear at the next boot, and the three choices to make there |
| At next boot | shim's key manager asks for the password at the console. This physical step cannot be automated, by design |
| If the user skips it | The system boots on the fallback graphics path. The desktop shows the enrolment state and offers to queue it again (D-025) |
| Certificate properties | Module-signing only: shim will not accept it for boot loaders or kernels (D-029, V-12) |

The user may decline enrolment in the installer. The choice is recorded and the consequence stated: the NVIDIA GPU will not be used until the certificate is enrolled.

Effect on Windows: the enrolled key is stored in shim's own variables and is consulted only when shim is in the boot path. Windows boots through its own boot manager, so its measurements should not change. "Should" is the honest word; this is criterion-tested on a disposable Windows install with BitLocker on before any dual-boot claim (V-22).

## 6. BitLocker and Windows device encryption

The installer cannot damage a BitLocker volume, because it never writes to one. The risk is different: Windows may ask for its recovery key after something about the boot environment changes, and a user without the key is locked out of their own data.

| Measure | Detail |
|---|---|
| Detection | Any BitLocker signature on any disk |
| Gate | Before the first write to any disk, the user must confirm that they have the recovery key. The screen explains where Windows shows it and that the installer cannot retrieve it |
| What ARKON does not do | Suspend or modify BitLocker; change Secure Boot state; alter the Windows ESP; alter Windows' firmware entry |
| What still can trigger a prompt | Firmware updates, firmware setting changes the user makes to boot the medium, and possibly boot-order changes on some firmware |
| Verification | V-22 |

Windows 11 Home enables device encryption automatically on many machines, so this applies to home users, not only managed laptops.

## 7. Signature enforcement on the installed system

The installed system must update only from ARKON-signed images (D-026).

- The orchestrator passes bootc's signature-enforcement option at deployment, with the target image reference taken from the image-shipped list.
- After deployment it reads back the recorded image source and fails the installation if enforcement is not recorded. A system that silently accepts unsigned updates must not be reported as a successful installation.
- Open conflict: bootc's documentation describes this option as requiring a signature-requiring default container policy. D-026 keeps a permissive default so that development containers can be pulled. Whether a scoped rule satisfies bootc is unverified (V-31). If it does not, D-026 needs a different construction, for example a restrictive default with explicit permissive entries for ordinary registries.

## 8. Encryption at rest and Secure Boot

Secure Boot verifies the boot loader and kernel. It does not verify the initramfs or the kernel command line under the GRUB chain, and it does not protect data at rest.

| Layer | Protection in 1.0 |
|---|---|
| Boot loader, kernel, modules | Verified |
| initramfs and kernel command line in `/boot` | Not verified. An attacker with physical access and time could alter them to capture a passphrase |
| Root filesystem | Encrypted if the user chose it; passphrase only (D-030) |

The installer states this in one sentence on the encryption screen rather than implying full protection. The fix is sealed boot after 1.0.

## 9. Privilege in the live environment

A live medium gives whoever boots it full control of the machine; the installer does not pretend otherwise. The security requirement applies to the installer's own interface:

- the orchestrator's API has no method that runs a supplied command, and no parameter that is passed to a shell;
- every tool is invoked with an argument vector built from validated, typed values;
- destructive execution requires the plan-bound confirmation (D-039), which protects against front-end defects and mistaken clicks, not against the person at the keyboard;
- the orchestrator binary is not part of the installed system.

## 10. Recovery procedures the user is given

| Situation | Procedure |
|---|---|
| Installed system does not boot with Secure Boot on | Boot the medium, run repair mode's diagnosis. Never: turn Secure Boot off |
| Enrolment screen was dismissed | Queue enrolment again from the desktop |
| Enrolment was lost after a firmware update | Same |
| Windows asks for the recovery key | Enter the key from the Microsoft account or the saved copy; ARKON's presence does not need to be undone |
| ARKON module key is revoked by the project | An update removes the old certificate and the desktop walks the user through enrolling the new one |
| Medium refused by firmware | Get a current ISO; check the firmware's third-party certificate option; update firmware |

## 11. Open points

| Point | Resolved by |
|---|---|
| Certificate transition on real firmware | V-29 |
| Enforcement option versus scoped policy | V-31 |
| No Windows recovery prompt after installation and enrolment | V-22 |
| shim rejects boot binaries signed with the module key | V-12 (Phase 1) |
| Whether enrolment can be queued from the live environment for a certificate inside a payload image, and survives to the installed system's first boot | Spike in stage 2 |
