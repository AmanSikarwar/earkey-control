# Initial architecture decision records

Date: 4 November 2026. These records separate approved constraints from conditional mechanisms. A proposed record is not an accepted hardware backend. References to specification sections mean the [unchanged v1.1 baseline](../requirements/Wired_Earphone_Module_Agent_Specification.md). Live sources and their limits are in [research](../research.md).

## ADR-001 Native interception versus overlays — proposed candidate

**Problem:** Suppress only earphone controls without harming shared Android mappings or audio. **Evidence:** Specification §§3, 6–7, 11; [Linux uinput](https://docs.kernel.org/input/uinput.html), [libevdev grab](https://www.freedesktop.org/software/libevdev/doc/latest/group__init.html), [AOSP layouts](https://source.android.com/docs/core/interaction/input/key-layout-files). No phone evidence exists.

**Behavior:** Evaluate exclusive evdev grab plus uinput from a mountless service. Stage 1 eligibility and Stage 4 passthrough are mandatory. Unknown or unsuitable nodes remain untouched. **Alternatives:** Read-only observation cannot cancel; grab-only is a separate reduced all-control mode; scoped layout overlays and framework adapters are deferred. **Revisit:** Measured transport, service permissions, or mapping proves the candidate unsuitable; document evidence and obtain acceptance before adding dependencies or changing baseline scope.

## ADR-002 Proxy identity and mapping — validation strategy accepted; mapping pending

**Problem:** Identical Linux codes may map differently on a virtual device. **Evidence:** Specification §7; [AOSP layout lookup](https://source.android.com/docs/core/interaction/input/key-layout-files). **Behavior:** Mark virtual devices distinctly, exclude them from discovery, advertise only verified capabilities, require Android readiness and mapping evidence, and assign one repeat owner before grabbing. **Alternatives:** Blindly clone identity or assume a delay proves readiness; both are insufficient. **Revisit:** Stage 4 requires a narrowly documented identity-field copy or scoped mapping; an overlay would change backend classification and needs a separate accepted decision.

## ADR-003 Conservative enrollment — accepted project constraint

**Problem:** Names and event numbers can select unrelated devices. **Evidence:** Specification §§4, 7, 17; physical identity still unmeasured. **Behavior:** Combine stable measured identity, collection, capabilities, and parentage, then revalidate after open and reconnect. Reject ambiguity, mixed essential input, general keyboards, and proxies. Model-level and serial-specific scope must be distinguished. **Alternatives:** Name-only, port-only, or event-number-only matching is prohibited. **Revisit:** A new transport needs new measured identity evidence and preserved denials, never a wider wildcard.

## ADR-004 Shared action semantics — accepted project constraint

**Problem:** Playback, calls, and assistant can share a center signal. **Evidence:** Specification §5; [AOSP USB headset mappings](https://source.android.com/docs/core/interaction/accessories/headset/usb-device) are references, not this accessory's measurements. **Behavior:** Block all and capability-driven Custom groups; disclose shared effects before activation. Unsupported distinctions retain the prior valid policy. **Alternatives:** Independent UI switches for one raw drop rule are misleading; gesture buffering and semantic injection are outside baseline. **Revisit:** Measured distinct signals support more independent groups, or a separately accepted semantic project has its own timing and compatibility tests.

## ADR-005 Independent supervision — accepted safety requirement; deadlines pending

**Problem:** A hang can retain an exclusive grab indefinitely. **Evidence:** Specification §§7, 13, 17. **Behavior:** Only the worker owns grab descriptors; a separate supervisor monitors real loop progress, bounds shutdown, rate-limits restart, and latches inactive after three failures per boot. Immediate pause and next-boot escape must precede the first physical prototype. **Alternatives:** Same-process watchdog or a heartbeat independent of forwarding cannot establish release. **Revisit:** Before Stage 4 choose measured deadlines and process identity/termination semantics; after fault injection refine bounds without weakening independent recovery.

## ADR-006 Durable policy and atomic generations — accepted storage choice; schema pending

**Problem:** Manager UI state cannot be the only durable policy and untrusted text must never execute. **Evidence:** Specification §§8–9; [KernelSU WebUI persistence caveat](https://kernelsu.org/guide/module-webui.html). **Behavior:** Plan `/data/adb/earkey_control/policy.json`, root-owned private storage, strict versioned JSON, atomic durable replacement, and worker acknowledgment of an exact generation. Keep durable pause/recovery state separately. No actual files or schema are installed. **Alternatives:** Shell-sourced config, sole localStorage, permissive schemas, and writable network control are rejected. **Revisit:** Stage 2 pins schema, size limits, migration rollback, and secure file-operation behavior; new managers remain adapters.

## ADR-007 Release identity, signing, and attestation — publication policy confirmed; release controls pending

**Problem:** Shipped bytes must match reviewed and physically accepted bytes without invented publisher identity. **Evidence:** Specification §§16, 18–20; [GitHub secure Actions](https://docs.github.com/en/actions/reference/security/secure-use), [immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases). **Behavior:** MIT/public source under AmanSikarwar is approved; release only after implementation and acceptance. Use an exact reviewed clean commit, pinned build inputs, deterministic packaging, SHA-256 manifest, two clean-build comparisons, dependency notices, available provenance, annotated tag and draft-first prerelease. Sign only with an established maintainer identity. **Alternatives:** Mutable assets, moved tags, source archives as installers, and rebuilding different bytes after device acceptance are rejected. **Revisit:** D08 must establish actual merge controls and signing/attestation availability before release; no release is eligible in Stage 0.
