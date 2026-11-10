# Initial architecture

The policy model, host event engine, supervisor state machine and [Android capability probe](prototype-preflight.md) are implemented. Android acquisition, the real process adapter, service wiring and physical passthrough still need validation. The approved stack is recorded in the [contract](project-contract.md); [decision records](decisions/0001-initial-design.md) explain the unresolved choices.

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

## Worker inventory

[earkey-worker](../crates/earkey-worker) starts with a read-only inventory, `earkey_worker::inventory::scan`. It never opens an input device.

- It walks `/sys/class/input/event*`, at most 64 nodes, and reads bounded UTF-8 attributes: the input name, `id/{bustype,vendor,product,version}`, the `ev`, `key` and `msc` capability bitmaps, and `properties`. Bitmaps are parsed as hex `unsigned long` words, most significant first.
- USB HID identity is derived only from the measured shape `usb-device/interface/BBBB:VVVV:PPPP.N/input/inputM/eventN`. That gives the USB vendor, product and `bcdDevice`; the interface number and class; the HID bus, vendor and product from the collection name; the serial; and the interface driver. Any other shape yields no USB identity, so it cannot match a USB profile.
- `is_virtual` means the node resolves under `/sys/devices/virtual`.
- `/dev/input/eventN` must be a non-symlink character device whose device number equals the sysfs `dev` value. Its filesystem device, inode and character device form the `NodeInstance`.
- Any node that cannot be fully observed fails the whole scan, so an unreadable duplicate cannot hide an ambiguity from the matcher.

An `Observation` lends itself to the policy matcher as a `Candidate`. The caller supplies the key layout digest.

`earkey_worker::environment` reads the pinned environment. The six properties `ro.product.model`, `ro.product.device`, `ro.build.version.sdk`, `ro.custom.version`, `ro.build.id` and `ro.build.version.incremental` come through bionic `__system_property_get`. The kernel release and build come from `/proc/version`. A missing, empty, control-character, possibly truncated (91-byte) or malformed value is an error, so an uncertain environment matches no evidence.

`earkey_worker::device` (Linux and Android) opens `/dev/input/<event>` read-only with `O_NONBLOCK`, `O_NOFOLLOW` and close-on-exec. The opened descriptor's filesystem device, inode and character device must equal the inventoried `NodeInstance`. It then re-reads the input ID, name, event-type, key, MSC and property bitmaps with `EVIOCGID`, `EVIOCGNAME`, `EVIOCGBIT` and `EVIOCGPROP`. The result, combined with the inventory's USB ancestry, is the opened candidate for `Policy::revalidate`. `pressed_keys` reports `EVIOCGKEY` for the neutral-state check. Nothing here reads events or grabs.

`earkey_worker::layout` hashes `system/usr/keylayout/Generic.kl` (a regular file of at most 64 KiB) with the pinned `sha2` crate. It treats a device-specific `.kl` or `.idc` (by vendor, product and version, or by canonical device name, in the ODM, vendor, system and data directories) as an override. A node with an override gets a digest that matches no evidence, so it cannot enroll or be targeted. The uinput proxy and the acquisition sequence are not implemented yet.

The reference evidence `xiaomi_pixelos20261107` pins the Stage 1 environment, including kernel `4.14.357-openela-VantomKernel`. After the maintainer installed the reforge kernel, a read-only [revalidation](discovery/reforge-revalidation-2026-11-10.md) found the accessory unchanged. `xiaomi_pixelos_reforge20261110` pins the new kernel with the same identity and capability rules. The kernel must be read from `/proc/version`, because `uname -r` on this kernel is spoofed to the old name, and the ROM comes from `ro.custom.version`.

## Host supervisor core

[earkey-supervisor](../crates/earkey-supervisor) is a host-tested state machine, not a process supervisor. It performs no process, clock, file, policy or device I/O. The service adapter supplies monotonic time, a gate built from durable pause, the pre-open `stay-inactive` marker and the restart latch, plus worker signals. It executes the returned actions.

The state machine starts only through an open gate. It requires an explicit startup acknowledgment and then a progressing event-loop counter; an unchanged counter past the progress deadline stops the worker. Orderly stop gets the pause deadline, then a forced termination gets the kill deadline. Failures are spaced by the restart deadline and counted within one supervisor session. The third failure requires a durable `PersistLatch` and forbids restart until explicit `clear_latch` after policy revalidation. If termination cannot be delivered or confirmed, the state is Unsafe: the adapter must persist the latch and never start another worker from that supervisor session.

The supervisor never starts or signals a child itself and never receives a grabbed descriptor. The adapter must enforce single-instance ownership, close-on-exec, verified process identity, and a progress channel updated by the actual worker loop rather than a separate thread. It must re-read the gate immediately before executing `Start`, and the worker must repeat the pre-open pause and marker check before opening a candidate. It must also terminate the verified worker process group, not merely the direct child, and confirm exit before treating a stop as complete. The core does not persist recovery state or report Active; it exposes phase, failure count and last fault for the service.

### Linux process adapter

`earkey_supervisor::process` (Linux and Android only) executes the state machine's actions for one worker:

- **Spawn:** the worker runs in its own process group with stdin and stderr closed and stdout as the progress pipe. A parent-death `SIGKILL` is set before exec, and spawn fails if the supervising thread already exited. Standard library spawning keeps other descriptors close-on-exec.
- **Progress frames:** each frame is 9 bytes, a tag byte and a little-endian `u64`. Tag 1 with counter 0 is the startup acknowledgment, and tag 2 is a progress counter. Frames are delivered to the state machine one at a time, so a backwards counter is never hidden inside a batch. A malformed frame, end of stream or an oversized backlog stops reading the channel; the progress deadline then stops the worker.
- **Stop and kill:** `Stop` sends `SIGTERM` to the group, and the worker must release its node and destroy its proxy on it. `Kill` sends `SIGKILL` to the group. A stop or kill with no worker is reported back as exited.
- **Exit confirmation:** the adapter observes the leader with `waitid(WNOWAIT)`. The unreaped leader keeps its PID and group ID reserved. It then sends `SIGKILL` to the group and scans `/proc` until no readable, non-zombie member remains, and only then reaps the leader. `/proc` entries denied by SELinux are skipped. That assumes the worker stays in the supervisor's own SELinux domain, and it must not leave its process group.
- **Gate:** an unreadable gate is treated as paused and boot-disabled. The gate is read again immediately before `Start`; if it closed, nothing is spawned and the following stop settles without counting a failure.
- **Latch:** `PersistLatch` and `Unsafe` call the host's latch write, and a failed write is returned to the caller.
- **Single instance:** an exclusive non-blocking `flock` on a caller-supplied lock file in validated private storage.

`earkey_supervisor::store` builds the gate from `Store::recovery` and the `stay-inactive` marker, and persists the latch with `Store::latch_restart_limit`. A store error keeps the worker stopped.

[earkey-drill](../crates/earkey-drill) is a bounded diagnostic, not the product worker. Its worker modes write progress frames and never open an input device. One mode is normal, one ignores `SIGTERM`, and one stalls after its first checkpoint. The `drill` command requires valid private storage and an open gate. It checks that the `supervisor.lock` lock is exclusive, then runs four scenarios against the store-backed gate: pause, kill escalation, stall detection and parent-death cleanup. It reports each duration, removes its lock file and never writes policy; its latch write always fails. The real worker and Android service wiring are not implemented yet. Its unsafe constructor requires that nothing else reaps the process's children and that the adapter lives on the supervising thread.

Start, progress, pause, kill and restart deadlines are constructor inputs and must be non-zero. They are not measured production values; Stage 4 must select and fault-test them with the real process adapter. The Linux process adapter and progress transport are host-tested. Android service wiring, the worker, the descriptor-release proof and the physical pause/recovery drill are not implemented or measured.

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
