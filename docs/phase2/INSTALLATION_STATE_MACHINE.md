# ARKON OS — Installation State Machine

Status: **DRAFT — design only.** Phase 2 is not open; see [PHASE2_ARCHITECTURE.md](PHASE2_ARCHITECTURE.md).

The orchestrator (`arkon-installd`) owns this state machine. The front end and the CLI only observe it and request transitions through the API. A request that is not valid in the current state is rejected; there is no way to jump to execution.

## 1. States

```
                         ┌────────────┐
                         │   Start    │
                         └─────┬──────┘
                               ▼
                         ┌────────────┐   legacy BIOS, no payload,
                         │ Preflight  │──────────────────────────────► Blocked
                         └─────┬──────┘   insufficient memory
                               ▼
                         ┌────────────┐   no eligible disk
        ┌───────────────►│  Scanned   │──────────────────────────────► Blocked
        │ rescan         └─────┬──────┘
        │                      ▼ CreatePlan
        │                ┌────────────┐   validator rejects
        ├────────────────│  Planned   │──────────────────────────────► (stays Scanned,
        │ change choice  └─────┬──────┘                                 error returned)
        │                      ▼ configuration and secrets supplied
        │                ┌────────────┐
        ├────────────────│ Configured │
        │ back           └─────┬──────┘
        │                      ▼ Confirm (hash + phrase; fresh scan matches)
        │                ┌────────────┐
        └────────────────│ Confirmed  │  token valid for a short time
          cancel / expire└─────┬──────┘
                               ▼ Execute
 ═══════════════════ nothing has been written above this line ═══════════════════
                         ┌────────────┐
                         │ Executing  │  steps X1…X12, no cancel after X3 starts
                         └──┬──────┬──┘
                    success │      │ failure
                            ▼      ▼
                     ┌──────────┐ ┌──────────────┐
                     │ Verified │ │   Failed     │ carries: step, error code,
                     └────┬─────┘ │              │ disk condition, cleanup options
                          ▼       └──────┬───────┘
                     ┌──────────┐        ▼ optional, explicit
                     │ Complete │   ┌──────────────┐
                     └──────────┘   │  CleaningUp  │──► Scanned or Failed
                                    └──────────────┘
```

| State | Meaning | Disk writes possible |
|---|---|---|
| Start | Service started | No |
| Preflight | Environment checks | No |
| Blocked | A condition prevents installation on this machine. Terminal except for rescan | No |
| Scanned | Storage report available | No |
| Planned | A validated plan exists | No |
| Configured | User, locale and, if chosen, encryption secret supplied | No |
| Confirmed | A confirmation token has been issued for this plan | No |
| Executing | Steps running | **Yes** |
| Verified | Result matches plan | Finalisation only |
| Complete | Ready to restart | No |
| Failed | Execution stopped | No |
| CleaningUp | Removing what this run created | Yes, limited to this run's own items |

## 2. Preflight checks

| Check | Failing result |
|---|---|
| Booted in UEFI mode | Blocked |
| 64-bit x86 CPU at Fedora's baseline | Blocked |
| Memory at or above the minimum | Blocked |
| Payload images present and their digests match the list in the live image | Blocked: damaged medium |
| Hardware report obtained; variant recommended | Warning if the GPU is unsupported for gaming; never blocks |
| Secure Boot and MOK state read | Informational |
| Power source | Warning on battery |
| Clock plausible | Warning only; nothing in an offline installation depends on it |

## 3. Confirmation (D-039)

| Plan type | What the user must do | What the orchestrator checks |
|---|---|---|
| Non-destructive (free-space install; re-create boot entry) | Press an explicit confirm control on the review screen | Plan hash matches; fresh scan equals the plan's scan |
| Destructive (erase disk; reinstall keeping home; reinstall boot files) | Additionally type a short phrase shown on the review screen, derived from the target disk's model and serial tail | As above, plus the phrase |
| Any plan on a machine with BitLocker | Additionally tick the recovery-key acknowledgement | Flag present |

The token is single-use, bound to the plan hash, and expires after a few minutes. Any change of choice, any rescan difference, or expiry returns the machine to `Scanned`. The front end cannot manufacture a token; a front-end defect can at worst fail to obtain one.

## 4. Execution steps

Each step is recorded in a journal before it starts and when it ends, together with identifiers of anything it created. The journal lives in memory and in the run log.

| Step | Action | Reversible | If it fails |
|---|---|---|---|
| X1 | Take sleep and shutdown inhibitors; pin the target disk by identifier | Yes | Stop; disk unchanged |
| X2 | Save the target disk's partition table and all firmware boot entries into the log | Yes | Stop; disk unchanged |
| X3 | Partition, format, encrypt (partitioner, real run) | **No in erase modes** — point of no return | See section 5 |
| X4 | Rescan and compare with the plan (validator, post-execution) | — | Stop before deployment |
| X5 | Unlock the encrypted volume if any; create subvolumes; mount root, `/boot` and the ARKON ESP at a private location; check the mounted ESP is the one created in X3 | Yes (unmount) | Cleanup option |
| X6 | Check the payload digest; deploy with `bootc install to-filesystem` | No | Cleanup option |
| X7 | Read back the deployment: image reference, signature enforcement recorded, loader entries present | — | Cleanup option |
| X8 | Write the first-boot handoff | Yes | Cleanup option; or continue to interactive first boot |
| X9 | Compare firmware entries with the X2 snapshot; apply boot order choice | Entry removal limited to the one created | Reported; installation is otherwise complete |
| X10 | Queue MOK enrolment if applicable and accepted | Yes | Reported as a warning; can be queued later from the desktop |
| X11 | Write the ARKON marker; copy the log into the target; unmount; lock the encrypted volume; flush | — | Reported |
| X12 | Final verification: remount read-only, check marker and layout, unmount | — | Reported |

Cancel is accepted in any state before `Executing`, and during `Executing` only before X3 begins. After that the run continues to a defined end, because stopping in the middle of partitioning or deployment leaves a worse state than finishing or failing cleanly.

## 5. Failure classes

| Class | Typical cause | Disk condition reported | Offered next |
|---|---|---|---|
| F-PRE | Preflight or scan problem | Unchanged | Guidance; rescan |
| F-PLAN | Validator rejected the plan | Unchanged | Change choices; if the rejection is rule P3 or P5, report as an installer defect |
| F-STALE | Disk changed between plan and confirmation | Unchanged | Rescan |
| F-PART | Partitioner failed (X3) | Erase mode: target disk incomplete, no other disk touched. Free-space mode: new partitions may partly exist, existing ones re-verified | Retry (erase mode); remove this run's partitions (free-space mode) |
| F-VERIFY | Post-execution comparison failed (X4) | **Unexpected change detected** | Stop everything. Show what differs, the saved partition table, and the manual restore procedure. No automatic repair |
| F-DEPLOY | bootc failed (X6, X7) | ARKON partitions exist but hold no bootable system | Retry deployment; or remove this run's partitions |
| F-HANDOFF | Could not write the handoff (X8) | ARKON installed; first boot will be interactive | Continue |
| F-FIRMWARE | Entry could not be created, or the comparison found a change (X9) | ARKON installed. Either not registered with firmware, or an existing entry differs | Fallback-path instructions; saved entry list for manual restoration |
| F-MOK | Enrolment could not be queued (X10) | ARKON installed | Queue later from the desktop |
| F-POWER | Power lost during execution | Depends on the step reached | On next start, section 6 |

Every failure report states, in this order: what failed, what condition each disk is in, whether other operating systems are affected, and what the user can do. "Other operating systems are not affected" is only stated when the post-execution comparison has actually run and passed.

## 6. Interrupted runs

The journal is in memory, so a power loss destroys it. Recovery relies on what is on disk:

| Found at next start | Interpretation | Offered |
|---|---|---|
| ARKON-labelled partitions without the marker | An installation that did not finish | Remove those partitions (free-space mode) or start a clean install over them |
| ARKON-labelled partitions with the marker | A complete installation | Normal choices and recovery modes |
| A firmware entry labelled ARKON pointing at a missing partition | Stale entry | Offer removal |
| Target disk with a damaged or empty partition table after an interrupted erase | Expected in erase mode | Start again |

The marker is written last (X11), which is what makes its absence meaningful.

## 7. Cleanup rules

Cleanup is never automatic. When the user asks for it:

- only partitions whose UUIDs are in this run's journal are removed, or, after a power loss, only partitions with ARKON labels and no marker;
- only the firmware entry this run created is removed;
- nothing on a disk other than the target is touched;
- the result is verified by another scan.

## 8. First boot

Not part of the orchestrator, but the continuation of the same flow.

```
 firmware ─► [MOK enrolment screen, if queued] ─► shim ─► GRUB ─► kernel
     ─► arkon-firstboot.service
           handoff present and valid ──► create user, apply settings, delete handoff
           handoff absent ─────────────► interactive first-boot setup
           handoff invalid ────────────► keep file, interactive setup, record reason
     ─► health checks mark the boot good (update design)
     ─► login screen ─► session agent: driver and enrolment state, default applications
```

## 9. Properties to be tested

| Property | How |
|---|---|
| No write step is reachable without a valid token | Model test over the transition table; fuzzing of the API in every state |
| A stale plan cannot be executed | Change the disk between plan and confirm in a loopback test |
| Existing partitions are untouched in non-erase modes | Content hashes before and after, in loopback tests and on a disposable Windows install |
| Every failure class leaves the stated condition | Fault injection at each step |
| Power loss at each step leaves a recognisable state | Kill the orchestrator at each step in loopback tests; pull power on bare metal for X3 and X6 |
| Cleanup removes only this run's items | Loopback tests with pre-existing foreign partitions |
