# Stage 0 review record

Date: 4 November 2026. **Status: Stage 0 exit verified. PR #2 is merged, issue #1 closed, private reporting enabled and the default-branch ruleset active.** No phone access, interception, prototype installation or release is part of this change.

## Concrete scope

Retain the complete supplied v1.1 requirements unchanged. Record approved identity/stack and maintainer decisions: MIT, owner AmanSikarwar, public earkey-control repository, release after implementation and acceptance. Prepare conventions/AGENTS.md, seven initial decision records, threat/failure model, compatibility levels/matrix, enrollment/policy/manager plans, recovery outline, full stage backlog, T01–T28 and performance/count validation plans, build/release/privacy/troubleshooting documents, dependency record and issue/PR templates.

No runtime source, workflow, installer, daemon, UI, device profile or generated binary is included. All compatibility entries are Design target. This review is not physical evidence or independent reviewer approval.

## Inspection and checks

- The selected workspace was empty, no repository/branch/remotes/history or ancestor/local AGENTS.md were present. User-supplied Context7 instructions were retained.
- Full specification read before creating files: 669 lines, 75,018 bytes, SHA-256 `eeeb157cc59986f27391598ebafb08988bed5fb420a928f1700853de4211dd4a`.
- Host tool/version inventory, configured Git identity presence and network-enabled GitHub identity/repository lookup completed; see research record. No ADB device enumeration or phone access occurred.
- PASS: retained specification matches the supplied file byte-for-byte and its recorded SHA-256.
- PASS: all 35 authored/retained files decode as UTF-8, use LF/final newlines and have no trailing whitespace; all 52 relative Markdown links resolve within the repository.
- PASS: backlog includes Stages 0–12, validation table contains T01–T28 in order, seven ADRs are present, six public issue templates have required front matter and a separate private-security route is defined.
- PASS: MIT license present; documentation inventory contains no Rust/shell/TypeScript/JavaScript runtime, workflow, installer or binary payload.
- PASS: Git ignore checks cover target/dist output, ZIP, private report/diagnostics/device evidence, environment secrets, phone image and UI dependencies.
- PASS: staged diff/stat/path list reviewed; all 35 staged blobs match their reviewed workspace files, no private/generated paths are staged, and staged whitespace check is clean. Repeat the affected checks after any further edit.

The initial structural-check invocation used a too-narrow stage-row expression and failed to recognize descriptive backlog rows. The corrected check recognizes the existing row format and passed without changing the backlog. No runtime test failed or was run.

## Repository and review artifacts

The first sandboxed Git initialization was denied `.git` access; approved metadata access then initialized local `main`. Empty bootstrap commit: `79570ed693ce19882923ac5a7c7f74a6e73da915` (`chore: initialize EarKey Control repository`), using the existing configured author. The documentation is isolated on `docs/stage-0-contract`; its final commit identifier is reported in the handoff/history to avoid a self-referential document hash.

At the initial handoff, network-enabled read-only GitHub access confirmed `AmanSikarwar` / `Aman Sikarwar`; selected repository lookup found no existing repository. The authorized creation attempt was rejected by GitHub: `Resource not accessible by personal access token (createRepository)`. No remote, upstream issue/PR, branch protection, required check, private reporting channel, tag or release was created during that attempt. A GitHub integration was discovered as an alternative but was not connected or claimed usable.

The maintainer subsequently supplied [the public repository](https://github.com/AmanSikarwar/earkey-control). Live checks confirmed default `main`, the HTTPS origin and baseline commit `9c1e00a19f0f74032be75bb85899ca8554e116b3` already published there. Preserve that shared history: the follow-up is on `docs/repo-setup`, fetched from `origin/main`, rather than fabricating a first-baseline diff. The initial handoff's 35-file baseline and seven ADRs remain unchanged apart from the setup records described below.

Created [acceptance issue #1](https://github.com/AmanSikarwar/earkey-control/issues/1). The setup follow-up was [PR #2](https://github.com/AmanSikarwar/earkey-control/pull/2), head `57d761a6701b4c85e1a8423f31d28fa4353c8984`. The maintainer subsequently merged it at `7c577198bf6db0c692e9064c064e8271f296c5e5` and closed issue #1; both states were verified through GitHub. No independent reviewer approval is inferred from the merge.

During PR #2 preparation, main-protection inspection and private-reporting enablement returned HTTP 403, no rulesets were returned, and reporting remained disabled. Those failed attempts remain evidence; protection was not overwritten and the issue chooser temporarily linked to SECURITY.md. They are superseded by the verified settings below.

The follow-up validation covers the retained specification digest/bytes, relative links, Stage 0–12 and T01–T28 coverage, issue-template metadata, text/whitespace, preservation of the published base and focused diff. Actual results and exact reviewed commit are recorded in the PR/handoff; runtime and device checks remain Not run.

## Verified Stage 0 exit

- PR #2 merged and issue #1 closed; clean local main fast-forwarded to the exact merge commit above.
- Private-reporting API returns enabled=true. SECURITY.md and the issue chooser can now direct reports to the private form; no test report was submitted.
- Active ruleset [Protection rules, 26527076](https://github.com/AmanSikarwar/earkey-control/rules/26527076) targets the default branch. Its details and effective main rules require PRs, prevent deletion/force pushes, require zero approving reviews and expose no bypass actors.
- The classic-protection endpoint returns 404 because no classic rule is configured; protection is supplied by the active ruleset. No destructive push/deletion test was attempted.
- No required-status-check rule or repository CI workflow is claimed. Add required checks alongside implemented jobs; merge/squash/rebase remain permitted. License, owner and release policy are confirmed, and all Stage 0 contract items are satisfied.

## Unresolved evidence and next step

No required Stage 0 product or repository-setup item remains pending. Exact phone/ROM/kernel/root implementation/ABI/accessory/transport/event mappings and service permissions are unmeasured. All runtime, physical T01–T28, loading, page-size, UI and reproducibility tests are Not run because implementation has not started.

The next eligible stage is read-only Stage 1 discovery. This chat has not authorized phone access or changes; obtain that explicit authorization before any ADB device enumeration or phone inspection. No tested support or safe recovery on the reference phone is claimed.
