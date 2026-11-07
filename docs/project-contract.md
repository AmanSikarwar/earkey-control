# Project contract

Date: 4 November 2026. Status: **Stage 0 exit verified; read-only Stage 1 discovery is authorized and incomplete. The maintainer explicitly approved publication of the reviewed sanitized checkpoint on 4 November.**

## Requirements and authority

The [supplied specification](requirements/Wired_Earphone_Module_Agent_Specification.md), version 1.1, is the acceptance baseline. Original SHA-256: `eeeb157cc59986f27391598ebafb08988bed5fb420a928f1700853de4211dd4a` (669 lines; 75,018 bytes). Its code-free rule applies to the specification, not future implementation. Direct user/platform instructions take precedence; record and surface substantive conflicts rather than silently weakening acceptance. No substantive conflict was identified in this review.

The maintainer selected MIT, owner `AmanSikarwar`, a public repository, and release only when everything is implemented. PR #3 is merged at `363bf13e10125cf75b19df7ef1fdd28340c1daec`. Read-only access to the wirelessly connected phone, including bounded earphone-candidate recordings, is explicitly authorized for Stage 1. After reviewing the concrete sanitized report, fixture and prepared issue, the maintainer replied "you can publish" on 4 November. This authorizes that checkpoint's public branch/PR and [acceptance issue #4](https://github.com/AmanSikarwar/earkey-control/issues/4). Raw diagnostics, serials/unique IDs and ADB network identifiers remain private. Installation, input interception/injection, root-manager/kernel changes, reboot and module-release publication remain outside the current authorization.

On 4 November the maintainer also approved publication of the follow-up reviewed at `7a384fe5fe95f38fb6707e56a10ad58ad7c75a15` by replying "publish". Its sanitized scope includes Center hold/double/cellular answer/end observations, listening/phone-control/USB-microphone and two-way call baselines, the diagnostic cleanup failure and correction, and the selected Center fixture. Raw audio, routing dumps, mixed event windows, process identities and failed helper revisions remain private; no additional phone action or module release is authorized by this publication approval.

On 5 November the maintainer approved publication of the screen-off/idle checkpoint reviewed at `61646a68f79e1cf8beb742ca9df0dd9a467eaecc` by replying "publish". Its sanitized scope includes the Dozing/locked/off short-button observations and fixture, the instrumented idle interval and reported whole-phone battery/CPU counters, their limitations and evidence digests. Raw endpoint/state dumps, process identities and private helpers remain private. This approval does not authorize further phone changes, interception or a module release.

On 5 November the maintainer also approved publication of the checkpoint reviewed at `47a6c60fa3fe7d2fc8ee3340dfee59cda73bc950` by replying "push". This covers the sanitized inline hold/double fixture, phone-side-key clarification, paired paused media-index observations and current eligibility review. Raw event/audio-service dumps, process identities and private analysis helpers remain private. No interception, additional phone change, release or Stage 1 exit is authorized by this publication approval.

On 5 November the maintainer approved publication of the screen-off volume and Center checkpoint reviewed at `e8c7921875a5d3a3904a1b69710a4078bdd36d51` by replying "push". This covers the three reviewed local documentation commits: volume holds/doubles with the four-plus-one group split, mixed Center double attempts, five individually reset Center holds with reported wake/assistant actions, state/reader endpoints and measurement limits. The two sanitized fixtures and evidence digests may be published. Raw diagnostics, unique identifiers, process details, assistant/audio content and private helpers remain private. This approval authorizes no further phone changes, interception, module release or Stage 1 exit.

On 6 November the maintainer approved publication of the seven local commits reviewed at `1be76a95d6c5ffe143c11d33464169f7e1f85118` by instructing "Push, update PR and Issue". This covers the sanitized per-group screen-off Center double/identity review, ongoing-call volume baseline, D10 maintainer-verification closure, six synthetic protocol cases, paused Center double/hold observations and paused-volume holds with their index discrepancy/mixed-input limits. Their reviewed source, reports and fixtures may be pushed to the existing draft PR #5 and open issue #4. Raw diagnostics, serial/network/process identifiers, audio and private helpers remain private. Stage 1 remains incomplete; no additional phone access, merge, tag, module release or interception is authorized by this publication instruction.

## Approved identity and stack

| Item | Approved value |
| --- | --- |
| Product | EarKey Control |
| Stable module ID | `earkey_control` |
| Approved public GitHub repository | [AmanSikarwar/earkey-control](https://github.com/AmanSikarwar/earkey-control); verified public, default branch main; origin is the supplied repository's HTTPS Git URL |
| Description | Configurable blocking of wired earphone button actions. |
| Platform order | KernelSU, then Magisk, then APatch |
| Android design range | Android 12 and later, beginning at API 31; no tested range yet |
| Reference setup | Redmi Note 10 Pro, PixelOS custom ROM, KernelSU, Xiaomi Type-C wired earphones |
| Core | Rust: discovery, policy, filtering, supervision, root command interface |
| Lifecycle | BusyBox ash |
| KernelSU WebUI | TypeScript compiled to JavaScript, HTML, CSS; bundled offline |
| Native builds | Cargo, pinned Rust toolchain, Android NDK |

Do not reopen these approvals without a concrete conflict or measured reason and maintainer agreement. No author identity, source license, phone codename, transport, ABI, event code, USB ID, manager fork, or kernel version is inferred.

## Product invariants

- Affect only explicitly enrolled, verified earphone control collections; unknown or ambiguous devices stay untouched. Mixed essential nodes and general keyboards are rejected.
- Preserve audio, microphone, routing, connection state, phone input, unrelated devices, and independent rescue controls. Audio-interface manipulation and global/shared layout edits are prohibited.
- Evaluate mountless exclusive evdev interception plus validated uinput forwarding; backend eligibility is unresolved until discovery. Prove passthrough before suppression. Reading alone is never presented as cancellation.
- Block all covers all verified button groups across an enrolled accessory's verified collections. Custom offers only independently measurable groups; shared center effects must be disclosed before activation.
- Fresh install is inactive until enrollment and explicit activation. Invalid configuration never widens scope. Pause, fault release, and next-boot inactivity must work without earphone buttons.
- SELinux stays enforcing. Start with no extra policy; any proven narrow permission needs independent review and service-context tests. No baseline mounting, injection, kernel changes, or network runtime dependency.
- Report requested, saved, and active policy separately. Preserve settings and pause state across updates. No security-boundary or zero-leakage claim is made.

## Decision register

| ID | Decision | Status / effect |
| --- | --- | --- |
| D01 | Source license and its final text | Confirmed 4 November: MIT; LICENSE contains full text |
| D02 | GitHub account/organization and selected remote | Confirmed 4 November: AmanSikarwar, public earkey-control; supplied repository and origin verified; published main contains baseline 9c1e00a19f0f74032be75bb85899ca8554e116b3 |
| D03 | Release/publication policy and who may approve publication | Confirmed 4 November: public source repository, release when everything is implemented; full physical, recovery and release acceptance gates still apply; no release now |
| D04 | Exact reference environment, accessory, transport, mapping, permissions | Root UID/context, userspace/kernel versions and USB/input/audio separation are measured in the checkpoint; remaining gesture/context/preservation and actual module-service evidence are pending. Reference reconnect repetition is waived by D09 |
| D05 | Native backend eligibility, proxy mapping, repeat owner, measured timeouts | Conditional candidate; settle through Stages 1–4 before active interception |
| D06 | Private security route | Enabled, verified by GitHub API on 4 November; SECURITY.md and issue chooser use the private reporting form. Earlier denied setup attempts remain historical evidence |
| D07 | Exact build pins, distributed ABIs, action revisions | Select and record before first corresponding build/workflow; installed tools are inventory only |
| D08 | Merge policy, account capabilities, available reviewers, signing/attestation identity | Active default-branch ruleset 26527076 requires PRs, blocks deletion/force pushes, has no bypass actors and zero required approving reviews. Merge/squash/rebase allowed; required CI rules will follow real jobs. Signing/attestation remains a later release gate |
| D09 | Reference reconnect repetition and Center clarification | Confirmed 4 November: no further physical reconnect-cycle runs for the reference setup; one observed cycle remains evidence. The maintainer confirms seven intentional Center presses, matching the seven captured pairs |
| D10 | Reference Stage 1 Center answer/end repetition closure | Confirmed 5 November: close the five-trial follow-up on maintainer verification of Center call behavior. One instrumented answer/end trial remains evidence; additional instrumented repetitions are waived, not represented as five captured passes. Other call contexts and module/release acceptance remain required |
| D11 | Reference Stage 1 ongoing-call inline volume holds | Closed on 7 November on the maintainer’s report that volume was the only action observed while holding the inline volume buttons during ongoing calls. The instrumented follow-up is waived for this item. Other call contexts and T08 remain open |
| D12 | Reference Stage 1 ongoing-call inline volume double presses | Closed on 7 November on the maintainer’s report that double-pressing a volume button during a call only displayed the volume slider. The instrumented follow-up is waived for this item. Other call contexts and T08 remain open |
| D13 | Reference Stage 1 ongoing-call Center holds | Closed on 7 November on the maintainer’s report that holding Center toggled call mute. The instrumented follow-up is waived for this item. T08 remains open |
| D14 | Reference Stage 1 ongoing-call Center double-press follow-up | Closed on 7 November at the maintainer’s request after they reported that pressing Center ended the call. The planned capture is waived. Tap count and double-press attribution remain unverified |
| D15 | Reference Stage 1 incoming-call Center holds | Closed on 7 November on the maintainer’s report that holding Center rejected the ringing call. The planned capture is waived. Timing, mapping and module acceptance remain unverified |
| D16 | Reference Stage 1 incoming-call Center double presses | Closed on 7 November on the maintainer’s report that double-pressing Center rejected the ringing call. The planned capture is waived. Tap timing, attribution and module acceptance remain unverified |

The maintainer resolved D01–D03 in this chat; no further Stage 0 product decision is requested. Other items have later explicit gates. Research ambiguity concerning APatch metamodules is recorded in [research](research.md); it does not justify altering the baseline.

## Reference validation amendment — 4 November 2026

The maintainer explicitly said "No need to perform reconnect cycles" and confirmed "I pressed 7 times" for Center. These direct instructions supersede the original reference-device repetition counts in specification §§4/17. Do not run further physical reconnect-cycle repetitions for this reference setup. Record one measured reconnect and the waived repetitions; never describe the original three-insertion or twenty-cycle thresholds as passed. Other environments retain their documented scope until changed explicitly.

The original specification remains byte-for-byte unchanged for provenance. This amendment changes the reference validation scope, not the software's required reconnect handling, stable matching, post-open/reconnect revalidation, essential-device denials or failure recovery. Host tests for those behaviors remain required. Seven intentional Center presses now explain the seven captured press/release pairs; no Center-count discrepancy remains. Other gesture, context, audio/microphone, isolation and service-permission evidence is still required.

## Reference Center-call validation amendment — 5 November 2026

When asked to prepare Center answer/end trial 2 of 5, the maintainer instructed: "mark all 5 completed, as I have verified pressing center intrupts or answers the call". Close the reference Stage 1 five-trial follow-up on that maintainer verification and do not request the four further instrumented repeat calls. This is a direct-user coverage amendment to the general five-observation discovery requirement, not five newly measured trials.

Evidence remains one instrumented incoming answer and ongoing end, plus the maintainer's verification of Center answering/interrupting a call. The user's instruction closes the repetition item; it supplies no additional event traces, trial-by-trial observations or numeric instrumented count. Preserve the original fixture and counts. The newly prepared trial-2 preflight started no reader or call. Additional recorded repetitions are waived rather than marked as measured passes; no assertion is made about how many unrecorded checks the maintainer performed.

This amendment applies only to the reference Stage 1 short Center answer/end repetition item. It does not establish unobserved call gestures, complete telephony routing, unrelated-input isolation, service permissions, proxy equivalence or recovery. Module T08/passthrough/suppression and final-artifact/release qualification still require their own physical evidence. Stage 1 remains incomplete. The original specification remains byte-for-byte unchanged.

## Reference ongoing-call volume-hold amendment — 7 November 2026

The maintainer reported that holding the inline volume buttons during ongoing calls changed only the volume while held. They asked to mark this item complete because they could not run the proposed test now. D11 closes the Stage 1 instrumented follow-up for ongoing-call inline volume holds on that report and waives the five-observation capture requirement for this item.

The report supplies a qualitative action observation. Trial count, call type, hold duration, raw event mapping and repeats, volume indices, call continuity, two-way audibility and microphone routing remain unmeasured for these holds. The existing instrumented short-press baseline is separate evidence. Incoming-call gestures, ongoing Center holds and double presses, ongoing volume double presses, and module T08 and release qualification retain their acceptance requirements.

## Reference ongoing-call volume double-press amendment — 7 November 2026

The maintainer reported that double-pressing an inline volume button during an ongoing call only displayed the volume slider. They reported that a test call was unavailable and asked to record the behavior and move on. D12 closes the Stage 1 instrumented follow-up and waives the planned capture repetitions for this item.

The report supplies a qualitative action observation. Button direction, trial count, tap timing, raw mapping, volume indices, call continuity and audio routing remain unverified for these doubles. Incoming-call gestures, ongoing Center holds and doubles, and module T08 and release qualification retain their acceptance requirements.

## Reference ongoing-call Center-hold amendment — 7 November 2026

The maintainer reported that holding the inline Center button during an ongoing call toggled mute and unmute. They said a test call was unavailable and asked to record the behavior and move on. D13 closes the Stage 1 instrumented follow-up and waives the planned capture repetitions for this item.

The report establishes the observed call action on maintainer verification. Hold timing, trial count, raw mapping and actual microphone-path behavior remain unmeasured. Ongoing Center double presses, incoming-call gestures, T08 and release qualification retain their own acceptance requirements.

## Reference ongoing-call Center double-press amendment — 7 November 2026

When asked to prepare the ongoing-call Center double-press check, the maintainer reported that a test call was unavailable and that pressing Center ended the call. They asked to record the behavior and move on. D14 closes the planned Stage 1 follow-up on that report and waives its capture repetitions.

Record call termination as the reported effect of a Center press. The number of taps, which tap ended the call and any distinct double-press action remain unverified. The earlier instrumented short answer/end trial supplies separate evidence under D10. Incoming-call gestures, T08 and release qualification retain their own acceptance requirements.

## Reference incoming-call Center-hold amendment — 7 November 2026

The maintainer reported that holding inline Center while an incoming call was ringing rejected the call. They said a test call was unavailable and asked to record the behavior and move on. D15 closes this Stage 1 follow-up and waives the planned capture repetitions for this item.

The report supplies a qualitative action observation. Hold timing, event mapping, trial count and audio routing remain unmeasured. Incoming Center doubles, incoming volume gestures, T08 and release qualification retain their own evidence requirements.

## Reference incoming-call Center double-press amendment — 7 November 2026

The maintainer reported that double-pressing inline Center while an incoming call was ringing rejected the call. They said a test call was unavailable and asked to record the behavior and move on. D16 closes this Stage 1 follow-up and waives the planned capture repetitions for this item.

The report supplies a qualitative outcome for the two-tap gesture. Tap timing, which tap produced the rejection, raw event mapping, trial count and call-state transitions remain unmeasured. Incoming volume gestures, T08 and release qualification retain their own evidence requirements.

## Stage 0 acceptance

- [x] Read the entire v1.1 specification and inspect selected workspace, instructions, Git state, and available tools before creating files.
- [x] Prepare reviewable contract, conventions, safety architecture/decisions, threat model, compatibility definitions, recovery outline, backlog, and validation plan.
- [x] Record approved identity and stack, prohibited behavior, evidence boundaries, and authorization limits.
- [x] Confirm source license, ownership, and release policy (D01–D03).
- [x] Check the documentation and prepare a local reviewable baseline. A draft PR is reviewable output, not claimed reviewer approval.
- [x] Verify the supplied public repository, published baseline and remote, and prepare an issue-linked documentation PR without rewriting published history. [PR #2](https://github.com/AmanSikarwar/earkey-control/pull/2) is merged at 7c577198bf6db0c692e9064c064e8271f296c5e5; [issue #1](https://github.com/AmanSikarwar/earkey-control/issues/1) is closed.
- [x] Verify private reporting is enabled and the active ruleset applies to main with a suitable solo-maintainer PR gate and deletion/force-push protection. No in-repository CI jobs or required status checks are claimed.

This checklist is not a stable-release definition. Specification Sections 17–20 remain the release gates. Read-only Stage 1 is authorized and in progress; its [checkpoint](discovery/reference-2026-11-04.md) does not close the stage or authorize a prototype. Independent host planning may continue, but hardware-dependent stages may not bypass discovery.
