# Fixture evidence and intended use

These fixtures support scoped discovery, host validation and a temporary service diagnostic.

| Directory | Evidence class | Intended use |
| --- | --- | --- |
| `discovery/` | Sanitized physical, unfiltered observations | Preserve exactly what was captured and what the maintainer reported; each record retains source/raw/helper identities and limits |
| `identity/` | Synthetic policy and serial | Exercise the reference evidence model without storing a physical accessory serial. Related host fixtures cover duplicates, mixed nodes, keyboards and a virtual proxy |
| `protocol/` | Hand-authored synthetic examples | Prepare independent host checks while device gates remain open; never count these events as physical observations |
| `resukisu-service-preflight/` | Temporary diagnostic package inputs | Exercise the reference late-start launcher with the reviewed metadata-only probe after device authorization. [Scope and cleanup](../../docs/prototype-preflight.md#temporary-service-diagnostic) |

The discovery report explains which physical windows qualify a baseline, which remain mixed or interrupted, and how D09–D20 change the discovery scope. The D11–D18 call gesture reports are recorded separately from the instrumented fixtures. [Discovery report](../../docs/discovery/reference-2026-11-04.md)

## Synthetic protocol summaries

[synthetic-event-summaries.v1.json](protocol/synthetic-event-summaries.v1.json) supplies six small examples: scan-bearing press/release, a scan-free release with nonzero SYN_REPORT, key-only input, an explicit held-key repeat, overlapping keys in one segment, and SYN_DROPPED. All times are invented for the examples; they are not measured latencies or gesture thresholds. Event constants refer to Linux namespaces, not Android keycodes or real-device capabilities.

Each case starts from the declared neutral state. Expected report-segment sizes include the terminating SYN_REPORT and retain its value. Key/scan counts summarize the raw data, including records ignored for state after a drop. Once SYN_DROPPED occurs, the final pressed-key set remains unknown until a state query; a visible later release cannot establish that state by itself. Raw segmentation is not permission to forward a segment affected by loss.

The summary format is test data, separate from the eventual policy/profile schema. Stage 3 [session tests](../../crates/earkey-engine/tests/session.rs) replay these records through the host engine and verify emitted events and the drop boundary. Additional synthetic cases exercise policy changes, partial reads/writes and output faults. Android forwarding and independent supervision remain later gates.

Primary references were consulted on 5 November 2026: [Linux input protocol](https://docs.kernel.org/input/event-codes.html) and [upstream v4.14 event constants](https://github.com/torvalds/linux/blob/v4.14/include/uapi/linux/input-event-codes.h). The latter supplies ABI constants, not exact source provenance for the custom reference kernel.
