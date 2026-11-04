# Threat and failure model

Status: design review, no implementation or fault-injection results. Scope follows specification §§6–13, 16–17. Protect normal physical input, audio/microphone/routing, boot availability, private policy/diagnostics, and the integrity of released bytes. Preserve physical phone volume/power rescue paths throughout.

## Trust boundaries

Untrusted surfaces include USB/device metadata and capabilities, evdev streams, imported profiles, policy files, CLI arguments, UI content, bridge requests, local IPC peers, filesystem paths/symlinks, and CI PR inputs. Root UID is not sufficient authorization for arbitrary operations. Root service access is further constrained by SELinux ([AOSP SELinux](https://source.android.com/docs/security/features/selinux)).

The trusted computing base includes the reviewed binaries, minimal syscall wrappers, supervisor, lifecycle hooks, parser/persistence operations, manager bridge, and pinned build chain. A hostile existing root process can defeat module controls and is outside the module's isolation guarantee. A malicious accessory can imitate identifiers; this convenience filter is not protection against malicious USB devices.

| ID | Threat or failure | Required containment / planned proof |
| --- | --- | --- |
| F01 | Wrong node, misleading name, event-number reuse, duplicate accessory | Multi-attribute enrollment and post-open/reconnect checks; protected-node denials; reject ambiguity; T02, T11, T13, T20, T22 |
| F02 | Mixed phone keys, switches, touch or general keyboard seized | Reject entire collection, regardless of profile request; T10, T11, T20 |
| F03 | Proxy matched recursively | Distinct virtual identity plus explicit exclusion, parent/capability checks; negative fixtures and T22 |
| F04 | Early grab, proxy unready or wrong Android mapping | Neutral state, bounded observed readiness, passthrough first, rollback partial acquisition; T04, T06–T08 |
| F05 | Held keys, policy change, repeat duplication, stray scan metadata | Per-press decision, frame/scan tracking, one repeat owner, safe transitions; T06, T14, T15 |
| F06 | Queue overrun, short read/write, lost output or overload | Kernel state resync or bounded release; bounded queues/retries; no log/UI stall while grabbed; T19, T26 |
| F07 | Worker crash, hang, inherited descriptor | Separate supervisor, close-on-exec and explicit descriptor ownership, progress deadlines, bounded terminate/kill, cleanup; T16 |
| F08 | Repeated restart or boot loop | Three-failure latch, persistent deliberate inactive reason, pre-open boot escape, non-blocking service; T12, T16, T28 |
| F09 | Other remapper/grab owner | Explain conflict, do not displace or defeat it, stay inactive; T18 |
| F10 | Malformed, huge, future or hostile policy / metadata | Strict parser, resource limits, no shell eval, no identity widening, data-only display; T05, T21 |
| F11 | Unauthorized control, shell injection or remote WebUI | Allowlisted operations, authenticated local IPC, validated arguments, text rendering, blocked external content/navigation, offline assets; T21, T27 |
| F12 | Path traversal, symlink swap, bad owner/type, unsafe cleanup | Controlled directories, descriptor-based validation, no arbitrary profile path, process identity verification, module-owned cleanup only; T21, T23, T24 |
| F13 | Full disk, concurrent writes, interrupted migration/update | Atomic durable replacement, generation serialization, validated backup, rollback rules, preserve pause; T15, T21, T23 |
| F14 | Root/SELinux denial or missing uinput | Probe actual service context; no broad allow/permissive workaround; inactive or explicit separately tested reduced mode; T17 |
| F15 | ROM/kernel/layout changes invalidate evidence | Bind profile evidence to environment/mapping constraints; revalidate or downgrade support; T25 |
| F16 | Diagnostics expose keypresses, identifiers, contacts or call content | Off by default, bounded candidate-only capture, preview/sanitize local exports, no upload, no raw public logs; T21, T27 |
| F17 | Untrusted CI code reaches rooted phone or release secrets | Reviewed controlled hardware runs only; PR read-only jobs; release isolation; full-SHA Actions and lockfiles; release review |
| F18 | Uninstall/manager toggle leaves active grab | Immediate pause, verified worker termination, manager-specific disable timing, independent recovery drill; T24, T28 |

## Unresolved safety parameters

Before physical interception, choose and test finite neutral acquisition, readiness, output stall, pause, supervisor heartbeat/termination and cleanup deadlines; bounded queue/log/file limits; and process identity checks. A proposal alone does not prove resource release. An uninterruptible kernel task may exceed userspace termination bounds: the prototype needs an independently tested module-disable/boot recovery plan, or it must not activate.

Fault recovery prioritizes restoring normal physical input over maintaining suppression. Normal buttons may reach Android while inactive, during startup/hotplug or after failure. Measure these windows; do not claim zero leakage. Stuck virtual state must be cleared or the proxy destroyed and the physical path restored, with the observed outcome recorded. Successful suppression cannot compensate for a preservation failure.

Privacy, independent recovery, and physical test procedures are in [privacy](privacy.md), [recovery](recovery.md), and [validation](validation-plan.md).
