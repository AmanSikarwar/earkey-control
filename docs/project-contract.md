# Project contract

Date: 4 November 2026. Status: **Stage 0 exit verified; the maintainer merged PR #3 and authorized read-only wireless ADB access for Stage 1. Discovery is incomplete; diagnostic publication awaits specific approval.**

## Requirements and authority

The [supplied specification](requirements/Wired_Earphone_Module_Agent_Specification.md), version 1.1, is the acceptance baseline. Original SHA-256: `eeeb157cc59986f27391598ebafb08988bed5fb420a928f1700853de4211dd4a` (669 lines; 75,018 bytes). Its code-free rule applies to the specification, not future implementation. Direct user/platform instructions take precedence; record and surface substantive conflicts rather than silently weakening acceptance. No substantive conflict was identified in this review.

The maintainer selected MIT, owner `AmanSikarwar`, a public repository, and release only when everything is implemented. PR #3 is now merged at `363bf13e10125cf75b19df7ef1fdd28340c1daec`. The latest instruction explicitly authorizes access to the wirelessly connected phone for the proposed read-only Stage 1 discovery, including bounded earphone-candidate recordings. Installation, input interception/injection, root-manager/kernel changes and reboot remain unauthorized. General public-source authorization does not override the specific diagnostic-publication approval gate raised during this collection; no Stage 1 phone findings have been posted.

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
| D04 | Exact reference environment, accessory, transport, mapping, permissions | Partial read-only measurements are in the local discovery checkpoint; repeat/context/root/service evidence and backend eligibility remain pending |
| D05 | Native backend eligibility, proxy mapping, repeat owner, measured timeouts | Conditional candidate; settle through Stages 1–4 before active interception |
| D06 | Private security route | Enabled, verified by GitHub API on 4 November; SECURITY.md and issue chooser use the private reporting form. Earlier denied setup attempts remain historical evidence |
| D07 | Exact build pins, distributed ABIs, action revisions | Select and record before first corresponding build/workflow; installed tools are inventory only |
| D08 | Merge policy, account capabilities, available reviewers, signing/attestation identity | Active default-branch ruleset 26527076 requires PRs, blocks deletion/force pushes, has no bypass actors and zero required approving reviews. Merge/squash/rebase allowed; required CI rules will follow real jobs. Signing/attestation remains a later release gate |

The maintainer resolved D01–D03 in this chat; no further Stage 0 product decision is requested. Other items have later explicit gates. Research ambiguity concerning APatch metamodules is recorded in [research](research.md); it does not justify altering the baseline.

## Stage 0 acceptance

- [x] Read the entire v1.1 specification and inspect selected workspace, instructions, Git state, and available tools before creating files.
- [x] Prepare reviewable contract, conventions, safety architecture/decisions, threat model, compatibility definitions, recovery outline, backlog, and validation plan.
- [x] Record approved identity and stack, prohibited behavior, evidence boundaries, and authorization limits.
- [x] Confirm source license, ownership, and release policy (D01–D03).
- [x] Check the documentation and prepare a local reviewable baseline. A draft PR is reviewable output, not claimed reviewer approval.
- [x] Verify the supplied public repository, published baseline and remote, and prepare an issue-linked documentation PR without rewriting published history. [PR #2](https://github.com/AmanSikarwar/earkey-control/pull/2) is merged at 7c577198bf6db0c692e9064c064e8271f296c5e5; [issue #1](https://github.com/AmanSikarwar/earkey-control/issues/1) is closed.
- [x] Verify private reporting is enabled and the active ruleset applies to main with a suitable solo-maintainer PR gate and deletion/force-push protection. No in-repository CI jobs or required status checks are claimed.

This checklist is not a stable-release definition. Specification Sections 17–20 remain the release gates. Read-only Stage 1 is authorized and in progress; its [checkpoint](discovery/reference-2026-11-04.md) does not close the stage or authorize a prototype. Independent host planning may continue, but hardware-dependent stages may not bypass discovery.
