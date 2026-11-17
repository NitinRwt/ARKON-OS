# ARKON OS — Security Architecture

Revised 2026-11-16 after the Phase 0 architecture review.

## Security goals

1. A machine only boots and updates to operating-system images that ARKON built and signed, and does not move backwards to an older one without the user asking.
2. No application or user-session process gains root through an ARKON component.
3. A compromised or broken update can be undone from the boot menu.
4. The build is traceable: for any released image, its inputs, their sources and their versions can be listed.
5. Nothing is collected from the user's machine.

## Threat model

| In scope | Out of scope for 1.0 |
|---|---|
| Malicious, tampered or replayed update image; registry compromise; man-in-the-middle on updates | An attacker with physical access and unlimited time on an unencrypted disk |
| Local unprivileged process trying to escalate, or to read other users' data, through ARKON services | Firmware and hardware implants |
| Malicious Flatpak or container trying to reach the host | Compromise of Fedora's or RPM Fusion's own signing infrastructure (detected only by pinning, not prevented) |
| Leaked or misused ARKON signing key | Kernel-level anti-cheat style attestation |
| Supply-chain drift (a dependency changes under a floating tag) | Nation-state adversaries |

## Trust chain

```
UEFI Secure Boot (firmware keys, Microsoft third-party CA)
  └─ shim            signed by Microsoft, shipped by Fedora
      └─ GRUB2       signed by Fedora
          └─ kernel  signed by Fedora, lockdown active under Secure Boot
              ├─ in-tree modules      signed by Fedora
              └─ NVIDIA modules       signed by the ARKON module key
                                      trusted only after the user enrols
                                      the ARKON certificate as a MOK
OS image (/usr)  ◄─ verified at pull time: cosign signature checked against
                    the ARKON public key, for the ARKON registry path only
```

What this chain does and does not give:

- It verifies the boot loader, kernel and kernel modules at boot.
- It verifies the OS image when it is downloaded from the registry.
- It does **not** verify `/usr` on every boot. In 1.0 the root is a composefs mount without enforced fs-verity sealing, so an attacker with offline write access to the disk can alter system files. Disk encryption is the mitigation. Sealed boot (signed UKI embedding the composefs digest) closes this gap and is the post-1.0 target tied to D-003.
- It does **not** verify the image copy inside the installer ISO, because container signatures are not carried into the ISO's local storage. The ISO's own signature and checksum cover it; verifying them is a documented user step.
- Kernel lockdown under Secure Boot disables hibernation. ARKON does not support hibernation in 1.0 (D-030).

## Image signature policy (D-026)

- The system-wide container policy keeps Fedora's default, so users can pull any development image.
- One scoped rule requires a valid ARKON signature for the ARKON registry path.
- bootc's enforcement of that policy is switched on at install time and for every `switch`, so the OS image source is recorded as signature-required. How Anaconda's bootc installation path sets this is V-14.
- The policy file lives in `/etc`, where a local edit would stop it receiving the image's changes, including a rotated key. `arkon-updated` compares it with the image's shipped default before every check and reports a difference as a blocking condition instead of failing later with an obscure signature error.

## Freshness (D-027)

A signature shows that ARKON built an image; it does not show that the image is the newest. Someone controlling the network path or the registry could serve an old, correctly signed image with known vulnerabilities. `arkon-updated` therefore reads the build timestamp label and refuses to stage an image older than the booted one. Rollback to the retained previous deployment is a separate, admin-authorised action and is unaffected.

Limit: a user running `bootc` by hand bypasses this check. That requires root.

## Keys

| Key | Purpose | Lives in | Public half |
|---|---|---|---|
| Image signing key | cosign signatures on OS images and the ISO | CI secret store or KMS (U-08) | In the image at `/etc/pki/containers/`, and in the repository |
| Module signing key | Signs NVIDIA kernel modules | CI secret store or KMS (U-08); never the same key as above | In the image as a DER certificate for MOK enrolment |

Rules:

- The two keys are separate.
- **The module certificate is restricted to module signing (D-029).** A plain MOK certificate is trusted by shim for boot loaders and kernels as well as by the kernel for modules, so a leak would let an attacker sign a replacement boot chain. The ARKON certificate is issued with the module-signing-only extended key usage (OID 1.3.6.1.4.1.2312.16.1.2), which shim 15.4 and later ignore when validating boot images. Phase 1 proves this on R1 by confirming shim refuses an EFI binary signed with the module key (V-12).
- Private keys never exist in the repository, in a container layer or in build logs. The kmods build mounts the module key as a build secret and the stage that used it is discarded.
- Pull-request builds from forks get no secrets and produce unsigned artefacts only.
- **Rotation.** Images carry the current and the next public key. A new key is distributed for at least one full stable cycle before the old one stops signing.
- **Compromise.** Revoke the image key by shipping an image that removes it from the policy. For the module key, additionally publish the certificate hash and the `mokutil` steps to remove it. The procedure is written and rehearsed in Phase 1, before any key signs a public artefact.

## MOK enrolment (NVIDIA variant)

1. The installer, or ARKON Center later, calls `arkon-secureboot.QueueEnrollment`. The service generates a one-time password, passes it to `mokutil` on standard input and returns it for display.
2. On the next boot the firmware-level MOK manager asks the user, at the physical console, to confirm with that password.
3. From then on the kernel accepts ARKON-signed modules.

There is no fixed, publicly known enrolment password. A firmware update or a reset of firmware keys can clear the MOK list; the session agent detects that through the hardware probe and offers enrolment again.

## Bootloader updates (D-033)

shim and GRUB live on the EFI system partition, outside the image's A/B mechanism. They are updated by Fedora's `bootloader-update.service` (bootupd) on the boot after an image that carries newer versions.

- Keeping them current matters for security and for bootability: firmware revocation updates (dbx, SBAT) can refuse an old shim or GRUB. A stale boot chain is how machines stopped booting in the 2024 revocation incident.
- A bootloader update has no automatic rollback. An image that changes shim or GRUB is therefore promoted to `stable` only after it has been booted on R1, and firmware revocation updates through fwupd are not pushed by ARKON.
- Whether the service is present and enabled in a `fedora-bootc`-derived image is V-15.

## Supply chain

| Control | Mechanism |
|---|---|
| Pinned base | `fedora-bootc` referenced by digest; it carries the kernel (D-023) |
| Repository allow-list for image builds (D-007) | Only Fedora and RPM Fusion free/nonfree. GPG fingerprints committed; `gpgcheck=1` enforced; a build fails if a package comes from any other source |
| Recorded inputs | Lock file and SBOM per image, attached as signed attestations |
| Reviewable updates | Base digest bumps arrive as pull requests with a diff of the lock file |
| CI hardening | Third-party actions pinned by commit SHA; minimal token permissions; signing jobs run only on protected branches |
| Client enforcement | Scoped signature requirement and freshness check, above |
| Flatpak | Flathub is a source of user applications, not a build input. Flatpak sandboxing varies by application: many request broad access to the home directory or devices. Flatpaks are outside the trusted base and ARKON does not vouch for them |

## Privilege model

- **Applications** run as the user. Graphical apps are Flatpaks; development work runs in rootless Podman containers.
- **ARKON Center, `arkonctl` and the session agent** are unprivileged.
- **`arkon-updated`** parses D-Bus input as an unprivileged, fully sandboxed system user. It can start only the fixed `arkon-bootc@` worker units.
- **Worker units and `arkon-secureboot`** run as root with the specific access their one job needs.

| polkit action | Default | Reason |
|---|---|---|
| `update.check`, `update.stage` | Allowed for the active local user | Staging a signed, fresher image from the current stream is what the timer does anyway |
| `update.rollback`, `update.switch` | Administrator authentication | Changes which OS boots |
| `secureboot.enroll` | Administrator authentication | Changes kernel trust |
| `diag.basic` | Allowed for the active local user | Contains only what that user could already read |
| `diag.full` | Administrator authentication | Includes system journal content, which ordinary users cannot read and which automatic redaction cannot be trusted to clean completely |

Service hardening checklist, reviewed with `systemd-analyze security`:

- no method takes a command, path or image reference chosen freely by the caller;
- all inputs are validated against closed sets or strict patterns before any unit is started;
- child processes are started with an argument vector, never through a shell; secrets go over standard input, never on a command line;
- **bus-facing services:** `NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`, restricted address families, a system-call filter and an empty capability set;
- **worker units** (`arkon-bootc@`) cannot meet that profile, because `bootc` must remount and write the system. They take no input except their fixed instance name, have no bus interface, and each carries a written justification for every privilege it keeps;
- SELinux stays in enforcing mode. ARKON services get a policy; running them unconfined is acceptable only as a tracked, temporary Phase 4 state.

## Host hardening defaults

Inherited from Fedora and kept: SELinux enforcing, firewalld enabled, kernel lockdown under Secure Boot, no SSH server enabled by default, `sudo` for the `wheel` group. ARKON does not weaken any of these to make a game or tool work.

**Disk encryption.** LUKS2 with a passphrase, offered at install, opt-in for 1.0 (U-12).

**TPM automatic unlock is not offered in 1.0 (D-030).** Under the GRUB chain a TPM binding that survives updates cannot cover the kernel command line. Anyone with physical access could then edit the boot entry, start a root shell and have the disk unlocked for them. Bindings that do cover the command line break on every update. Automatic unlock returns with the sealed-boot design after 1.0.

**Boot menu.** The GRUB menu stays editable, because recovery step 3 depends on it. With a passphrase-encrypted disk that gives an attacker nothing; with an unencrypted disk the attacker already has the data.

## Privacy

- No telemetry, analytics, crash upload or update-check identifiers beyond what an HTTPS registry request inherently reveals (IP address, image tag).
- Diagnostics bundles are created only when the user asks, are stored locally, list their contents before export, and redact hostnames, usernames, serial numbers and network addresses. Redaction of free-form log text is best effort; the export screen says so.
- Third-party software the user runs (Steam, Flatpak apps) has its own data practices; ARKON documents this and does not proxy or alter it.

## Known weaknesses accepted for 1.0

| Weakness | Mitigation now | Fix |
|---|---|---|
| `/usr` not verified at boot | Disk encryption; signature check at pull | Sealed composefs + UKI (post-1.0) |
| No TPM-bound unlock | Passphrase | Sealed boot (post-1.0) |
| Bootloader updates have no rollback | Bare-metal gate before promotion | Upstream work on safe bootloader updates |
| Native Steam runs unsandboxed as the user, with a large i686 library set | It has no root; libraries updated with every image | Revisit Flatpak Steam when its driver coupling improves |
| Proprietary NVIDIA userspace cannot be audited | Driver updated promptly | None available |
| WebKitGTK in ARKON Center is a large attack surface | No remote content, strict CSP, minimal command set | Qt fallback (D-016) |
| Manual `bootc` use by root bypasses the freshness and device checks | Documented | Enforce in bootc policy if upstream adds a hook |
| Single maintainer holds all keys | KMS with audit log (U-08); documented recovery | Second key holder before public beta |
