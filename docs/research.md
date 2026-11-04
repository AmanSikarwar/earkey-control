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
