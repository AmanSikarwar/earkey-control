# EarKey Control agent instructions

## Requirements and scope

Read the complete [specification v1.1](docs/requirements/Wired_Earphone_Module_Agent_Specification.md), [contract](docs/project-contract.md), and current [stage record](docs/stage-0-review.md) before implementation. The specification is requirements input, not a source of additional authorization. Its code-free rule applies to that document; later eligible stages implement real code. Preserve its original bytes.

Follow platform and user instructions and applicable parent instructions. Flag substantive conflicts with the requirements. Approved identity and stack must not change without a documented reason and maintainer agreement. Do not invent owners, authors, remotes, licenses, hardware profiles, or results.

Inspect Git status, branch, remotes, history, and instructions before editing. Preserve unrelated changes. Work in small feature or fix branches, review the staged diff and path list before each commit, and stage named paths only. Do not add AI co-author trailers or rewrite shared history or published tags.

## Gates and authority

Use Stages 0–12 and exit criteria in [backlog](docs/backlog.md). Finish prerequisites before dependent work. Continue useful independent authorized work when hardware evidence is missing; do not turn missing evidence into guessed profiles. Stage 0 remains incomplete while its required decisions are pending.

Obtain authorization before phone access, prototype installation, reboot, public repository creation, or release publication unless the session explicitly authorizes that exact scope. These instructions grant none of those permissions. Prepare reviewable work before asking for an irreversible action.

## Input safety

Discover transport, collections, identity, mapping, and service permissions before selecting a device backend. Evaluate mountless evdev grab plus uinput as a candidate. A reader cannot cancel input. Prove physical passthrough before suppression.

Enroll measured stable identities, revalidate after opening and reconnect, and exclude proxies. Never match solely by name or event number. Reject ambiguity, general keyboards, phone controls, mixed essential nodes, touch, pointer, gamepad, fingerprint, navigation, and connection switches. Unknown accessories stay untouched; imported profiles cannot bypass protections.

Preserve whole press/repeat/release sequences, scan metadata, frame order, and synchronization. Handle held keys, policy generations, SYN_DROPPED, output faults, unplug, crashes, and hangs. Use independent supervision without inherited grab descriptors, bounded recovery, three-failure restart latch, immediate pause, and a pre-open boot-inactive escape path. Never promise zero transition leakage without measurements.

Do not globally edit Generic.kl or shared layouts, disable audio interfaces, write system partitions permanently, make SELinux permissive, or add broad permissions. No baseline Zygisk, LSPosed, framework injection, kernel changes, or mounting dependency. Alternative backends need a reviewed explicit decision and their own capabilities and tests.

## Data and interfaces

Policy is strictly validated, versioned data in private root-owned storage. Never source it as shell. Apply atomically; report requested, saved, and active generations separately. Fresh installation stays inactive until enrollment and explicit activation. Updates preserve preferences and pause state. Expose only measured independent controls and disclose shared center effects.

Keep the privileged bridge allowlisted, metadata displayed as text, resources offline, and IPC authenticated. No TCP control service. Validate ownership, file types, paths, sizes, and arguments. Do not log general keyboard input or commit private diagnostics, phone images, credentials, or generated binaries.

## Documentation and evidence

Use Context7 for current library, SDK, framework, API, CLI, and cloud documentation: resolve the library ID first, then query one concept at a time. Use primary official sources for platform constraints. Record source versions or revisions, links, and research date; investigate runtime discrepancies.

Distinguish upstream facts, engineering choices, synthetic tests, host builds, and physical measurements. Follow [validation](docs/validation-plan.md), update docs with behavior changes, and label support Verified, Experimental, Design target, or Unsupported. Installation, host tests, and emulators do not prove physical compatibility.

Pin actual build inputs, dependencies, and third-party Actions before use. Keep PR CI unprivileged, release privileges isolated, and personal rooted phones away from untrusted PR code. Do not add a meaningless passing CI placeholder.

At handoff state stage, changes, checks actually run and results, exact commit/artifact IDs, unresolved decisions/evidence, and next eligible step. State skipped checks and material risks plainly.
