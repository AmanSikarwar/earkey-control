# Initial architecture

The policy model, host event engine and supervisor state machine are implemented. Android acquisition, the real process adapter, service wiring and physical passthrough still need validation. The approved stack is recorded in the [contract](project-contract.md); [decision records](decisions/0001-initial-design.md) explain the unresolved choices.

## Candidate and boundaries

Evaluate an ordinary mountless module service that starts an independent Rust supervisor and a Rust input worker. Only the worker owns evdev grab and uinput descriptors. The supervisor launches, monitors actual event-loop progress, and terminates a stalled worker; it must never inherit or retain a grabbed descriptor. The command interface and UI do not handle input descriptors.

| Component | Responsibility and trust boundary |
| --- | --- |
| Policy engine | Pure validation, capabilities, group decisions, immutable policy generations; no manager or device I/O |
| Discovery/enrollment | Bounded observation, transport correlation, stable measured matching, deny checks, revalidation after open/reconnect |
| Worker | Exclusive acquisition, frame/state reconciliation, validated forwarding, per-sequence suppression, bounded cleanup |
| Supervisor | Single-instance ownership, independent deadline, boot escape, restart accounting and inactive latch |
| Root command interface | Defined operations, validated input, peer/owner checks, atomic persistence, exact-generation acknowledgment |
| Manager adapter | Version/capability detection, portable installation/service/removal hooks; no policy logic |
| Offline KernelSU WebUI | Capability-aware controls, accessible status, immediate pause, fixed bridge operations; no durable policy in localStorage alone |

The actual implementation may use a small number of binaries/subcommands; these boundaries do not require a crate or service per row. Avoid adding libevdev or another dependency until the audited syscall requirements justify it. The kernel documentation recommends considering libevdev for correctness; dependency choice remains open ([uinput documentation](https://docs.kernel.org/input/uinput.html)).

## Eligibility before acquisition

Stage 1 must prove the accessory's actual transport and dedicated key-only collections. Type-C does not establish USB HID. Match measured bus, vendor/product where present, interface/collection, name, capability fingerprint, and parent relationship. Event numbers and USB ports are runtime details. Reject ambiguity, mixed essential nodes, switches, general keyboards, phone input, and the module's virtual identity. Profiles cannot override those denials.

Permission probes must run in the intended module service domain with SELinux enforcing. A root shell or accepted policy rule is insufficient. Unknown accessories remain untouched. No automatic fallback to reduced or mounted backends is permitted.

## Acquisition and active processing

1. Check durable pause/next-boot inactive state and recovery latch before opening a candidate. Load a validated immutable policy; confirm no competing worker.
2. Open defensively, revalidate the opened identity/capabilities and protected-device exclusions, and inspect physical key state. Wait for neutral within a deadline; otherwise remain inactive.
3. Prepare only measured proxy capabilities with an unmistakable virtual identity. Verify Android layout, classification, HID scan mappings, policy flags, and one repeat owner. Require a bounded observed readiness acknowledgment; a fixed sleep is insufficient.
4. Acquire the exclusive grab only after forwarding is ready. Drain/reconcile queues and recheck key state; if transition correctness is uncertain, release and retry from neutral within limits.
5. Report Active only after all enrolled collections required for the requested policy are ready. Roll back partial accessory acquisition on failure; never report Block all while a required collection remains unintercepted.
6. Preserve ordering, complete frames, scan association, and each press decision through repeats and release. New policy decisions apply only to new sequences or a safe neutral transition. No delayed gesture recognizer in the baseline.
7. On SYN_DROPPED, ignore through the next SYN_REPORT, query current state, and reconcile forwarded keys; if equivalence cannot be proven, stop interception. Handle short/interrupted reads and short/failed writes, multiple keys per frame, bounded backpressure, removal, and malformed events.
8. On pause/fault/removal, clear forwarded pressed state where possible, destroy proxies, close/ungrab the physical node, and acknowledge the resulting inactive state. If orderly pause stalls, independent supervision must bound release.

Linux input is stateful; press/release/repeat and overrun rules come from [kernel event documentation](https://docs.kernel.org/input/event-codes.html). Android virtual-device equivalence is a separate physical gate because layout selection depends on identity ([AOSP layouts](https://source.android.com/docs/core/interaction/input/key-layout-files)).

## Host event engine

[earkey-engine](../crates/earkey-engine) consumes normalized Linux event records from a nonblocking source and writes complete-record prefixes to a nonblocking sink. The caller supplies the session clock and starts only after verifying neutral physical and proxy states. Native adapters will use the Linux UAPI layout for the target ABI and reject partial native records. The host engine does not open or grab devices.

The engine buffers one input batch and one output frame, up to 64 records each. It validates the whole frame before writing its first record. It retains each allowed key record, its preceding scan if present, and the original report record. A fully suppressed frame emits no report. A new generation is selected at the next frame boundary, and each held key retains its press decision through repeat and release. The engine forwards source repeats as received and synthesizes none. Android and the proxy still need a physical repeat-ownership check.

The session exposes an incomplete-frame or output deadline to its caller and fails after eight consecutive interrupted operations. It advances past each accepted output prefix and waits for sink readiness on backpressure. The caller must wake it by the deadline even if the descriptor stays quiet. Tests use a 100 ms stall limit. Stage 4 must select and verify the production deadline together with independent supervision.

On `SYN_DROPPED`, the engine discards the partial frame and consumes through the next report within the frame and deadline bounds. It queries current key state, then returns a terminal fault. A snapshot alone cannot reconstruct lost press decisions or scan association. The engine reports which keys may be down on the proxy for cleanup. The worker must destroy the proxy and release the physical node, then start any retry from a fresh verified neutral session. [Linux input protocol](https://docs.kernel.org/input/event-codes.html)

## Host supervisor core

[earkey-supervisor](../crates/earkey-supervisor) is a host-tested state machine, not a process supervisor. It performs no process, clock, file, policy or device I/O. The service adapter supplies monotonic time, a gate built from durable pause, the pre-open `stay-inactive` marker and the restart latch, plus worker signals. It executes the returned actions.

The state machine starts only through an open gate. It requires an explicit startup acknowledgment and then a progressing event-loop counter; an unchanged counter past the progress deadline stops the worker. Orderly stop gets the pause deadline, then a forced termination gets the kill deadline. Failures are spaced by the restart deadline and counted within one supervisor session. The third failure requires a durable `PersistLatch` and forbids restart until explicit `clear_latch` after policy revalidation. If termination cannot be delivered or confirmed, the state is Unsafe: the adapter must persist the latch and never start another worker from that supervisor session.

The supervisor never starts or signals a child itself and never receives a grabbed descriptor. The adapter must enforce single-instance ownership, close-on-exec, verified process identity, and a progress channel updated by the actual worker loop rather than a separate thread. It must re-read the gate immediately before executing `Start`, and the worker must repeat the pre-open pause and marker check before opening a candidate. It must also terminate the verified worker process group, not merely the direct child, and confirm exit before treating a stop as complete. The core does not persist recovery state or report Active; it exposes phase, failure count and last fault for the service.

Start, progress, pause, kill and restart deadlines are constructor inputs and must be non-zero. They are not measured production values; Stage 4 must select and fault-test them with the real process adapter. The adapter, progress transport, Android service wiring, descriptor-release proof and physical pause/recovery drill are not implemented or measured.

## Failure and state reporting

Expose Inactive, Waiting for earphones, Discovering, Ready, Active, Unsupported, or Fault, with a reason, requested/saved/active generations, selected/effective backend, and recovery action. Ready means eligibility, not suppression. A running process is not evidence of Active. Paused and latched states are inactive with an explicit reason.

Use event-driven input/hotplug waiting and bounded rescans after notification races. Never hold a grab during unbounded logs, UI calls, output retries, or a blocking queue. A heartbeat must depend on the worker loop progressing, including writes and cleanup; an independent heartbeat thread is insufficient. On three failures in one boot, latch inactive. A clean restart can reset the boot counter but must honor any persisted deliberate latch or pause; explicit recovery must validate policy again.

Exact neutral, output, pause, heartbeat, kill, and reacquisition deadlines are unresolved engineering parameters, to be fixed and tested before Stage 4. No indefinite wait is acceptable. Physical tests must measure actual release time and startup, hotplug, policy, and recovery windows. Pre-activation presses can leak; this is a convenience filter, not a malicious-device security boundary.

## Alternatives and preservation limits

| Backend | Eligibility / status |
| --- | --- |
| Grab + uinput | Primary candidate, only after service access and Android passthrough validation |
| Grab without forwarding | Optional all-control-only mode on a verified key-only node; separate explicit choice and tests; not implemented |
| Device-specific layout overlay | Deferred; exact ROM resolution, complete unrelated mapping preservation, mounting provider and reload proof required |
| Framework semantic adapter | Outside baseline; accepted dependency model, versioned semantics, recovery, and new decision required |
| No interception | Required outcome for unknown, ambiguous, denied, mixed, or unproven devices |

Never modify Generic.kl globally, replace shared layouts, take audio nodes, disable USB audio interfaces, change drivers/kernel/root manager, weaken SELinux, or add baseline mounting or injection. Hardware-local volume and route-removal pauses may be outside the observed input path and cannot be promised as blockable.
