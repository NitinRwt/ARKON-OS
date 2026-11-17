# ARKON OS — Phase 1 Remediation Plan

Date: 2026-11-16
Follows from [PHASE1_AUDIT.md](PHASE1_AUDIT.md). Verdict: Phase 1 not approved, because Phase 1 has not been implemented.

There is nothing to repair. The remediation is to carry out Phase 1 as already planned in TASKS.md, in the order below, and to pass the gate that precedes it first. No working Phase 1 module exists, so none is rewritten.

## Blockers before Phase 2

Phase 2 cannot begin until every item here is closed with evidence in TEST_RESULTS.md.

| ID | Blocker | Closed when | Owner |
|---|---|---|---|
| B-01 | Phase 0 gate has not passed | U-01…U-07 answered and D-001…D-033 approved or rejected, recorded in DECISIONS.md (criteria 0.7, 0.10) | Project owner |
| B-02 | No version control, licence or CI | Git repository with a remote, licence chosen, CI running lint on every pull request (P1-01, P1-02) | Engineering, after U-02 and U-07 |
| B-03 | No build host | Builds run in CI; optionally Podman on the development machine (P1-02) | Engineering |
| B-04 | No image definition | Both variants build from a clean checkout with one command each; base pinned by digest; lock file and SBOM produced; allow-list enforced; lint passes (criteria 1.1–1.4, 1.10, 1.15) | Engineering |
| B-05 | No NVIDIA module pipeline or signing | Modules built for the image's exact kernel, signed with a module-only certificate; mismatch fails the build; shim rejects a boot binary signed with that key (1.6, 1.14) | Engineering, after U-05 and U-08 |
| B-06 | No image signing or client enforcement | Signed images; unsigned and wrongly signed images refused on upgrade and switch; ordinary container pulls unaffected (1.5) | Engineering, after U-08 |
| B-07 | No bare-metal test target | Test disk chosen and prepared; pre-install checklist done on the laptop (Windows disk-encryption status and recovery key, backup, boot order, Secure Boot state) | Project owner (U-01, P0-09) |
| B-08 | Nothing has ever booted | The NVIDIA variant boots to a Plasma Wayland session on the reference laptop with Secure Boot on, with and without the certificate enrolled; upgrade and rollback succeed (1.7–1.9) | Owner performs installs; engineering records results |
| B-09 | Key procedures do not exist | Rotation and compromise procedures written; rotation rehearsed with test keys (1.11) | Engineering |
| B-10 | Open verification items | V-01…V-04 and V-10…V-15 resolved; systemd-boot spike written up; module flavour measured and decided (1.12, 1.13, 1.16–1.18) | Engineering |
| B-11 | A phase was reported complete with no evidence behind it | From now on a phase is called complete only when TEST_RESULTS.md holds, for each acceptance criterion, the image digest, the hardware, the date and the result | Everyone |

## Order of work

1. **Close the Phase 0 gate (B-01).** Nothing below starts before this. The decisions are listed in PROJECT_STATE.md.
2. **Prepare the laptop (B-07).** Independent of engineering work and can be done in parallel with step 3.
3. **Repository and CI (B-02, B-03).** P1-01, P1-02.
4. **Base and desktop stages (B-04).** P1-03, P1-04, P1-05, P1-14. First real answers to V-01 and V-07.
5. **Boot chain facts.** P1-06: bootloader, boot counting, automatic bootloader updates (V-02, V-03, V-15).
6. **NVIDIA (B-05).** P1-07, P1-08, P1-09 with test keys.
7. **Signing (B-06, B-09).** P1-11, then P1-12 for custody and production keys.
8. **Gaming packages.** P1-10, install only.
9. **First boot and hardware tests (B-08).** P1-15 to P1-18, with the owner at the machine.
10. **Spike and measurements (B-10).** P1-13, P1-19.
11. **Phase 1 review and gate.** P1-20. Then rerun this audit.

Roadmap estimate for steps 3–11: four to six engineer-weeks, unvalidated.

## Non-blocking issues

None are recorded, because there is no implementation to have minor issues. The open design decisions that Phase 1 itself is meant to settle (U-08, U-14, U-18) are part of B-05, B-06 and B-10.

## Re-audit criteria

The next Phase 1 audit should be requested only when all of these are true:

- the repository is under git and the audited commit is named;
- CI has a green run for that commit that built both variants;
- TEST_RESULTS.md has an entry for each of criteria 1.1–1.18 with an image digest;
- at least one entry records a boot on the physical reference laptop.

The audit will then repeat the fifteen checklist questions against the built image and the installed system.
