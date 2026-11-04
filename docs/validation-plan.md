# Validation plan

Status: test design; **all module runtime and physical acceptance cases are Not run**. Partial unfiltered Stage 1 baseline observations and retained diagnostic failures are in the [discovery checkpoint](discovery/reference-2026-11-04.md). The maintainer approved its sanitized publication; it is not a module acceptance pass. The unchanged specification §§4, 7, 16–17, 20 controls acceptance.

## Evidence rules

Each result records case ID, UTC execution time, operator, source commit and reviewed state, artifact SHA-256, complete environment, profile/mapping-evidence version, backend, requested/saved/active generation, expected outcome, observed outcome, Pass/Fail/Not run/Not applicable, count and evidence location. Not applicable needs a concrete capability-based reason; missing hardware is Not run. Keep raw private evidence outside Git; publish only reviewed sanitized extracts. Distinguish upstream documentation, engineering choice, synthetic fixtures, host execution, emulator loading, and physical measurements.

For physical environments record variant, Android/API, full ROM, kernel release/build, root implementation/fork and kernel/userspace/manager versions, executable ABI, page size, enforcing SELinux/service domain, accessory identity/transport/collections and Android mappings. Verify physical results against the release bytes or an identical recorded digest. Changing code, build flags or packaged bytes after acceptance requires the affected tests again.

An allowed event must cause its intended action exactly once without added action. A blocked event must not cause its targeted action in supported contexts. Audio, microphone, routing and unrelated controls need separate observations; silence does not prove preservation. Any wrong target, input loss, broken audio/microphone, unsafe policy or unreliable recovery blocks stable publication.

## Host and packaging checks to implement

| Area | Meaningful cases / gate |
| --- | --- |
| Policy/parser | Valid Block all/Custom, missing/unknown/duplicate/future fields, corrupt and oversized text, bounded depth/lists, namespace errors, unsupported splits, duplicate profiles, capability widening; fuzz corpus and regressions |
| Identity | Measured valid candidate, same names/different parent or capabilities, changing event number, symlink/node reuse, ambiguous duplicates, composite collections, mixed switches/touch/phone keys, keyboard with media keys, own proxy |
| Event state | Whole press/repeat/release sequences, overlaps and multiple keys per frame, scan ownership, held-key generation change, single repeat owner, interrupted/short reads and writes, malformed values, overload, disconnect, SYN_DROPPED state query/reconciliation and safe fault |
| Lifecycle/security | Single worker/descriptor ownership, progress-based hang detection, pause/kill deadline, three-failure latch, boot escape, migration interruption/full storage/concurrent commands, peer credentials, path/owner/type validation, hostile metadata |
| Manager adapters | Explicit supported/missing/conflicting environment fixtures; actual service behavior remains a device test; preserve pause/update data, verified-process cleanup |
| UI | TypeScript/static build, defined bridge operations/arguments, text-only metadata, blocked remote content/navigation, offline resources; screen reader, focus, contrast, touch targets and ≥200% text scaling |
| Builds | Formatting/static analysis, BusyBox ash syntax/lint, unit/migration tests, parser fuzz regressions, applicable sanitizers, UI validation; unsupported sanitizer environments explicitly explained |
| ELF/package | Every distributed ABI: Android ELF machine/interpreter/imports/API minimum, loading, page alignment; ZIP root metadata/executable modes/LF, inventory, no wrapper directory/private data/unknown payloads/network executable fetch; version consistency |
| Reproducibility | Two clean builds of exact reviewed commit, pinned dependencies/tools, normalized archive entries/modes/times/order, compare final digests; unexplained difference blocks byte-reproducibility claim |

A final required CI status must fail when a mandatory dependency fails or is unexpectedly skipped. Hardware evidence remains separate; no mock or CI label substitutes for the real suite.

## Physical T01–T28 matrix

All rows below are **Not run for a module; no runtime artifact exists**. Authorized baseline observation does not change those outcomes. Stage numbers identify the earliest relevant evidence; later lifecycle/release qualification repeats affected cases on final bytes.

| ID | Case | Acceptance | Earliest stage |
| --- | --- | --- | --- |
| T01 | Fresh install | Inactive before enrollment and explicit activation | 6 |
| T02 | Unknown accessory | Untouched, clear enrollment option | 2 host; 4 physical |
| T03 | Block all | All observed verified button actions suppressed in supported contexts | 5 |
| T04 | Each Custom group alone | Selected blocked; independent unselected groups behave normally once | 5 |
| T05 | Shared center semantics | Collateral effects disclosed; unsupported splits unavailable and prior valid policy retained | 2 host; 5 physical |
| T06 | Short, long, repeat, double | Complete sequences and documented gestures correct, no duplicate repeats | 3 host; 4 passthrough; 5 blocking |
| T07 | Screen on, locked, off | Supported mapping/wake behavior correct; limits disclosed | 4–5 |
| T08 | Incoming/ongoing calls | Expected policy, audio/mic route preserved with consenting/local setup | 4–5 |
| T09 | Music/microphone recording | Playback/recording equivalent to baseline; no audio node interference | 4–5 |
| T10 | Phone volume/power | Fully usable, including rescue paths during interception | 4 |
| T11 | Unrelated input/keyboard | Unchanged, including unrelated media keys | 4 |
| T12 | Boots with/without accessory | No boot delay/wrong target; correct eventual state | 6 |
| T13 | New event number after reconnect | Stable identity safely revalidated and policy reapplied | 4–6 |
| T14 | Unplug with held key | No stuck virtual key/resource leak; normal physical input returns | 4 |
| T15 | Policy change mid-press | Decision unchanged for sequence; generations accurate | 3 host; 5–6 physical |
| T16 | Worker crash/hang | Bounded release/recovery, no persistent input loss; deadline measured | 4 safety; 7 full faults |
| T17 | Missing uinput/denied permission | Safe inactive or explicit separately tested reduced mode | 4–7 |
| T18 | Existing other grab | Precise conflict, no displacement or attempt to defeat it | 4–7 |
| T19 | SYN_DROPPED/output failure | Correct state recovery or bounded inactive state | 3 host; 4–7 physical |
| T20 | Mixed node/misleading name | Rejected; essential switches/controls remain untouched | 2 host; 4 physical |
| T21 | Hostile policy/metadata | Safe rejection, no execution/widening/exposure | 2–3 host; 6–7 integrated |
| T22 | Duplicate/composite collections | Only enrolled collections; ambiguity leaves inactive | 2 host; 4 physical |
| T23 | Update/migrate/downgrade | Preferences/pause retained, no duplicate worker; backup rollback tested | 6–7 |
| T24 | Pause/disable/uninstall | Immediate pause; actual manager timing documented; no hidden process | 4 pause; 6–7 lifecycle |
| T25 | ROM/kernel update | Evidence invalidation/revalidation, unsupported state where appropriate | 10 |
| T26 | Idle/reconnect stress | Budgets measured; no busy loop, log flood, descriptor/memory leak | 4 baseline; 6–7 stress |
| T27 | Accessible offline UI | Screen reader/focus/contrast/≥200% scaling; settings/pause without network or earphone buttons | 6 |
| T28 | Independent recovery drill | Maintainer can disable only this module through pretested independent path | Before/during 4; repeat 6–7 |

## Required counts and environments

- Discovery: at least five observations of each mapping and three insertion cycles, preserving complete event/action evidence.
- Release qualification: at least 20 press/release cycles per supported independent group, 20 reconnect cycles, and three boots in **each** tested boot configuration (with and without earphones). Document tested combinations; add stress for a regression or timing concern.
- First evidence is the user's exact KernelSU implementation on the reference setup. Upstream KernelSU and each fork need separate rows. Magisk and APatch each require real event, lifecycle and preservation evidence on their own environments.
- API 31 must be physically tested before claiming the minimum. Test the newest stable release being claimed and state intermediate coverage. Additional digital headsets, USB adapters, and any analog backend each need their own enrollment and preservation evidence.
- Verify phone controls and an unrelated keyboard/other available input, including media keys; expand Bluetooth isolation separately. Do not change the primary phone's root method/kernel to fill a matrix.
- Build and load on 4 KiB and 16 KiB systems before those claims; emulator loading is useful executable evidence, not proof of physical earphone or root-manager compatibility.

## Performance and transition measurements

Project budgets are idle CPU **<0.1%** over a defined five-minute sample, worker memory **<15 MiB**, and added passthrough latency **p95 <20 ms** on the reference phone. These are acceptance targets, not results or upstream guarantees.

Proposed method: compare matched baseline and active five-minute idle samples with identical screen/audio/connection conditions, recording process CPU time divided by elapsed wall time as percentage of one core; report supervisor separately and aggregate wake sources. Sample worker RSS and report peak/steady state without conflating cache/PSS. Use monotonic physical-receipt and proxy-emission timestamps to measure per-frame forwarding delay; separately correlate Android action timing against unfiltered baseline so queue delay and framework latency are not hidden. Record sample count/distribution, instrument clock/overhead and tracing effect. Final instrumentation must be validated before claiming the latency budget.

Measure startup, hotplug, policy-transition, failure and recovery inactive windows and immediate-pause/grab-release time, including held keys and output stalls. Choose finite safety deadlines before Stage 4 and fault-test them; this specification does not supply measured values. No persistent wakelock or high-frequency polling. Investigate every idle wake source.
