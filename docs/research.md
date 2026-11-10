# Research and local inspection record

Research/inspection date: **4 November 2026**. This is upstream documentation and host evidence, not phone evidence. The supplied specification's S01–S31 bibliography retains its original 3/4 November dates. Only the sources explicitly listed as refreshed below were fetched again for this task. No runtime version is inferred from a website's current branch.

## Current primary sources consulted

| Source / revision scope | Fact used / limit |
| --- | --- |
| [KernelSU module guide](https://kernelsu.org/guide/module.html); Context7 `/tiann/kernelsu`, guide on `main` | Ordinary service scripts are distinct from system mounting and use a non-blocking service stage; scripts do not inherently require a metamodule. Installed fork/version remains unmeasured. |
| [KernelSU WebUI](https://kernelsu.org/guide/module-webui.html); current official page | Local webroot entry/bridge exists, manager-applied permissions should be retained, and localStorage alone is not durable. Exact bridge package/API will be pinned and rechecked at implementation. |
| [Magisk development guide](https://topjohnwu.github.io/Magisk/guides.html); Context7 `/topjohnwu/magisk`, `master` guide | Supported customization and service hooks, metadata and BusyBox ash conventions; similar packaging does not prove manager identity or physical compatibility. |
| [APatch APM guide](https://apatch.dev/apm-guide.html); official page and Context7 `/bmax121/apatch` | APatch-specific environment/version markers, paths and lifecycle exist. Do not use Magisk-like directory identity alone. Exact service version/context is unmeasured. |
| [APatch metamodules](https://apatch.dev/meta-module.html); current official page | Page exists but its introduction describes KernelSU; this is documentation ambiguity, not a verified APatch-version capability contract. |
| [Linux event codes](https://docs.kernel.org/input/event-codes.html); rolling kernel docs | Stateful frames, key values and overrun recovery constrain event processing. Pin actual relevant kernel/header definitions before native implementation. |
| [Linux uinput](https://docs.kernel.org/input/uinput.html); rolling kernel docs | Userspace can create virtual input devices; libevdev is recommended for consideration. This does not establish Android proxy mapping. |
| [libevdev initialization/grab](https://www.freedesktop.org/software/libevdev/doc/latest/group__init.html); latest docs | Grab/ungrab uses EVIOCGRAB. Reading a stream alone cannot provide exclusive cancellation. No libevdev dependency has been selected. |
| [AOSP key layouts](https://source.android.com/docs/core/interaction/input/key-layout-files); current official page | Layout selection uses device identity/name and mappings include policy flags. Global Generic.kl modification is prohibited. Actual ROM selection must be measured. |
| [AOSP USB headset device mappings](https://source.android.com/docs/core/interaction/accessories/headset/usb-device); page displays update 9 October 2026 | Standard center signal has contextual actions. This is not evidence that the Xiaomi accessory uses that transport or those codes. |
| [AOSP SELinux](https://source.android.com/docs/security/features/selinux); current official page | Mandatory policy also constrains root processes. Service-domain evidence is still required. |
| [Rust Android targets](https://doc.rust-lang.org/rustc/platform-support/android.html); current rustc book | Android ELF cross-compilation uses supported Android targets/NDK; installed host rustc is not a reproducible pin. |
| [NDK newer APIs](https://developer.android.com/ndk/guides/using-newer-apis); current official page | Native API availability/imports affect loading on older supported releases. API 31 is the project's design floor. |
| [Android page sizes](https://developer.android.com/guide/practices/page-sizes); current official page | NDK version/link choices, ELF alignment and fixed-page assumptions require inspection and runtime testing. APK archive alignment is separate from root module ZIP packaging. |
| [KernelSU rescue](https://kernelsu.org/guide/rescue-from-bootloop.html), [Magisk FAQ](https://topjohnwu.github.io/Magisk/faq.html), [APatch rescue](https://apatch.dev/rescue-bootloop.html); current official pages | Manager-specific mechanisms have version/integration/timing qualifications; none is proven for this phone or a specific fork. |
| [GitHub protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches); current docs | Required checks/reviews and restriction availability depend on repository/account; avoid impossible solo self-approval. |
| [GitHub secure Actions](https://docs.github.com/en/actions/reference/security/secure-use); current docs | Minimize token privileges, isolate untrusted jobs and pin third-party Actions by reviewed full SHA. No workflow exists yet. |
| [GitHub immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases); current docs | Published assets/tags can become immutable; assemble a complete draft first. No release feature is assumed enabled. |
| [GitHub repository REST API](https://docs.github.com/en/rest/repos/repos); Context7 `/websites/github_en_rest` | Administrators can enable private vulnerability reporting; check actual permission and resulting state, not just an assumed UI route. |
| [GitHub branch-protection REST API](https://docs.github.com/en/rest/branches/branch-protection); refreshed through Context7 `/websites/github_en_rest` on 4 November 2026 | A PR gate may use zero required approving reviews for a solo maintainer; required status checks can remain unset until jobs exist. Inspect existing protection before replacing it. |

Context7 resolution preceded each query. Official upstream repositories were preferred over forks: KernelSU `/tiann/kernelsu`, Magisk `/topjohnwu/magisk`, APatch `/bmax121/apatch`; GitHub `/websites/github_en_rest` was used for private reporting. These are documentation indexes, not immutable implementation dependencies. No moving documentation branch is claimed as a pinned build input.

## Discrepancies and open verification

APatch's metamodule page uses KernelSU wording, while APM documentation and indexed upstream source describe APatch-specific environment/lifecycle behavior. This review cannot establish the user's APatch version (none is provided), or a universal mounting contract. At Stage 9 inspect the exact supported version's source/release and real runtime before selecting adapter behavior; if an overlay is ever accepted, independently verify its mounting provider. The mountless candidate avoids assuming this disputed behavior but still needs service/permission tests.

Kernel documentation examples use simplified readiness waits; project requirements instead require bounded observed Android readiness before a grab. This is a deliberate stricter engineering requirement, not a contradiction or proof that an arbitrary sleep works.

## Host inventory and boundaries

Selected directory was empty, including hidden entries, with no `.git`, branch, remote, source, or local/ancestor AGENTS.md found on the inspected path. The user supplied Context7 instructions apply and are retained in AGENTS.md. An initial sandboxed Git initialization failed because `.git` creation is protected; later approved metadata operations must be recorded in the stage review.

| Available tool / environment | Observed value |
| --- | --- |
| Host | macOS 27.2, arm64 |
| Git | 2.54.0 (Apple Git-157) |
| GitHub CLI | 2.103.3 (2026-10-26) |
| Rust / Cargo | rustc 1.99.1 (`58cf8f9c3`, 2026-10-13); cargo 1.99.1 (`5f94df478`, 2026-08-27) |
| Node / npm | v20.17.0 / 11.1.0; compatibility with a future selected UI build is not checked |
| ADB executable | 1.0.41, platform tools 37.0.1-15733141; version queried only, no server/device enumeration or phone access |
| Android NDK directories | 28.2.13676358 (`r28c`) and 30.0.16138531 with source.properties identifying **30.0.16138531-beta3 / r30-beta3**; neither is a project pin |
| Other tools | Python 3, clang, ShellCheck, zip/unzip, SHA-256 utility and rustup on PATH |
| Not on PATH | BusyBox, llvm-readelf, standalone TypeScript compiler, Markdownlint and Prettier; not a claim that NDK-bundled tools are absent |
| Git author | Existing name/email configured; no identity was created or changed |
| GitHub identity | Network-enabled read-only API confirmed login `AmanSikarwar`, name `Aman Sikarwar`; requested repository lookup found no existing repository |

Sandboxed GitHub calls initially could not reach the API and reported an invalid token. The network-enabled read-only check authenticated successfully; the earlier message was not treated as proof of invalid credentials. An authorized public creation attempt then reached GitHub and was denied with `Resource not accessible by personal access token (createRepository)`. Authentication and repository-creation permission are separate facts. No credential value was printed or stored. Repository setup results belong in [Stage 0 review](stage-0-review.md).

No toolchain installation, Android compilation, USB/ADB phone probe, SELinux change, input capture, grab, proxy creation, audio test, call, reboot or release has occurred in this stage. Sources referenced only by the original specification, including exact ROM EventHub/KeyboardInputMapper revisions, still require implementation-time verification.

## Supplied-repository follow-up

After the maintainer supplied the public repository URL, live GitHub and Git checks on 4 November verified `AmanSikarwar/earkey-control`, public visibility, default branch `main`, matching origin URL and published main at `9c1e00a19f0f74032be75bb85899ca8554e116b3`. The local checkout was clean on main and had no unique unpublished baseline work. A fetch confirmed the base before creating `docs/repo-setup`; no published history was rewritten. Initial PR/issue lists were empty; acceptance [issue #1](https://github.com/AmanSikarwar/earkey-control/issues/1) was then created.

Repository permissions report the authenticated account as admin and the owner is the only direct collaborator; this does not imply the token has every administration scope. Main-protection inspection returned HTTP 403 (`Resource not accessible by personal access token`), while ruleset listing returned an empty array. Existing main protection cannot therefore be asserted absent or safely replaced. Private-reporting GET returned false, its enable PUT was denied HTTP 403, and a subsequent GET remained false. No test security report was submitted, no contact invented, and no required status check configured without an implementation. The existing merge options were preserved.

## Post-merge verification

After the maintainer reported completion, live checks on 4 November confirmed PR #2 merged at `7c577198bf6db0c692e9064c064e8271f296c5e5`, issue #1 closed, and private-reporting GET returning true. Local main was fast-forwarded to that exact commit from a clean checkout. The earlier denied attempts above are retained as history, not current blockers.

The classic main-protection endpoint now returns 404 (`Branch not protected`), but ruleset `26527076`, named Protection rules, is active and targets the default branch. Its rules and the separate effective-rules-for-main response both include deletion, non_fast_forward and pull_request. PR approvals are set to zero, bypass actors are empty, and the current user cannot bypass. Merge/squash/rebase are allowed; no required-status-check rule is present. These are inspected settings, not destructive enforcement tests or a claim that CI exists.

Context7 `/github/docs` was resolved and queried for ruleset semantics on 4 November. [GitHub's ruleset documentation](https://github.com/github/docs/blob/main/content/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets.md) explains that rulesets and classic protection are separate mechanisms that can coexist. A classic-rule 404 therefore does not invalidate the independently observed active ruleset. No phone access, runtime testing or test vulnerability submission occurred.

## Publication tooling refresh — 6 November 2026

GitHub CLI 2.103.3 (2026-10-26) was queried locally for this publication. Context7 `/websites/cli_github_manual` was resolved, then queried for description edits through the official [gh pr edit](https://cli.github.com/manual/gh_pr_edit) and [gh issue edit](https://cli.github.com/manual/gh_issue_edit) manuals. Both support `--body-file`; prepared UTF-8 files supply complete descriptions without shell interpolation. Actual remote heads and PR/issue state are checked separately; documentation is not evidence of a successful push or retained draft/open state. No new device research or phone access is part of this refresh.

## Stage 2 policy and storage research — 7 November 2026

- Serde 1.0.229 and serde_json 1.0.154 supply typed JSON decoding. The policy decodes the original bytes to reject duplicate fields and checks object shape to exclude positional struct forms. The Rust tests verify these rules on the locked versions. Context7 resolved `/serde-rs/serde` for strict derive behavior. [Serde attributes](https://serde.rs/container-attrs.html), [serde_json 1.0.154](https://docs.rs/serde_json/1.0.154/serde_json/)
- Rust 1.99.1 and libc 0.2.191 supply file locking, synchronization and descriptor-relative filesystem operations. Anchored lookups use fixed names and reject symlinks. The design synchronizes the directory as a separate durability step after rename. Host tests exercise the implementation on Darwin. Android manager paths and the actual storage filesystem still require integration tests. [Rust File API](https://doc.rust-lang.org/std/fs/struct.File.html), [libc 0.2.191](https://docs.rs/libc/0.2.191/libc/), [Linux openat](https://man7.org/linux/man-pages/man2/openat.2.html), [Linux fsync](https://man7.org/linux/man-pages/man2/fsync.2.html)
- The identity model uses the reviewed reference observations for allowed capabilities and keeps Linux codes, HID usages and Android mappings in separate types. The inventory representation uses sorted, complete capability sets and fails closed on differences. [Linux input event codes](https://docs.kernel.org/input/event-codes.html), [reference discovery](discovery/reference-2026-11-04.md)

## Stage 3 event and host test research — 7 November 2026

- Linux evdev defines stateful key values and the report boundary for discarding events after an overrun. Upstream v4.14 evdev reads use the native input-event size and can return fewer complete records than requested. The host interface counts normalized complete records. Android adapters must validate their actual ABI and partial-write behavior. [Linux event codes](https://docs.kernel.org/input/event-codes.html), [Linux v4.14 evdev source](https://github.com/torvalds/linux/blob/v4.14/drivers/input/evdev.c)
- uinput can emit the source values through an initialized virtual device. Matching Android layout, action, and repeat ownership remain separate physical gates. The host engine retains source repeats and stops after synchronization loss when the key-state snapshot cannot prove continuity. [Linux uinput documentation](https://docs.kernel.org/input/uinput.html)
- The host checks use a pinned Rust nightly and sanitizer. Context7 resolved `/rust-fuzz/cargo-fuzz` for campaign setup and bounded arguments. The independent fuzz lockfile keeps test-only dependencies out of the module workspace. [Rust sanitizer support](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html), [cargo-fuzz 0.13.2](https://github.com/rust-fuzz/cargo-fuzz/releases/tag/0.13.2)

## Stage 4 capability probe research — 7 November 2026

- The first probe uses the locally installed archived NDK r28c revision `28.2.13676358` with the Rust 1.99.1 Android target. Context7 resolved `/websites/developer_android_ndk` for the target-prefixed Clang driver. The official SDK index supplied the Darwin ZIP digest, and the installed tool hashes are recorded separately. Target-specific C and Rust layout checks use the NDK Linux UAPI and pinned libc 0.2.191. [Build record](build.md#stage-4-android-capability-probe), [NDK external build systems](https://developer.android.com/ndk/guides/other_build_systems), [Rust Android targets](https://doc.rust-lang.org/rustc/platform-support/android.html)
- Linux uinput separates opening its character device from creating a virtual device. The open-only diagnostic checks the measured 10:223 identity and closes the descriptor without issuing ioctls. [Linux v4.14 uinput source](https://github.com/torvalds/linux/blob/v4.14/drivers/input/misc/uinput.c), [uinput documentation](https://docs.kernel.org/input/uinput.html)
- The historical ReSukiSU repository URL redirects to BakaSU. GitHub resolves the measured userspace short revision to `516d3e87fbc986ccefdde2c2765f1700612441a9`. That source dispatches the services event to the `service` stage, finds the active module script and launches it through BusyBox. Custom kernel source provenance and the live service SELinux context remain unresolved. [Fork revision](https://github.com/Baka-SU/BakaSU/commit/516d3e87fbc986ccefdde2c2765f1700612441a9), [Fork module launcher](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/module/mod.rs)

- The same pinned fork revision exports `KSU`, `BAKASU`, the module ID and userspace, kernel and UAPI version markers to installer and service scripts. Boot-mode installation stages the ZIP under `modules_update`; uninstall marks the specific module for boot-time pruning. Active metamodules can override installation or run a removal hook, so the diagnostic requires checking delegation before staging. BusyBox provides the timeout applet used by the late-start fixture. [Fork module implementation](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/module/mod.rs), [Fork installer](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/module/installer.sh), [BusyBox applets](https://busybox.net/downloads/BusyBox.html)
