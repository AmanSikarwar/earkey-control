# EarKey Control

Configurable blocking of wired earphone button actions.

**Stage 0 verified; authorized read-only Stage 1 discovery is in progress. A local [discovery checkpoint](docs/discovery/reference-2026-11-04.md) records partial physical evidence and remaining gates; its publication awaits approval. No module, installer, daemon, UI, or enrolled profile exists. Nothing here is installable.**

The product will offer Block all and Custom controls for explicitly enrolled earphones while preserving audio, microphone use, connection detection, phone controls, and unrelated input devices. Controls are independently selectable only when measured signals permit it. A shared center signal may also control calls, playback, and assistant gestures.

KernelSU is the first design target, followed by Magisk and APatch. Android 12 / API 31 and later is the design range. **No Android version, root manager, or accessory is verified.** The reference setup is a Redmi Note 10 Pro, PixelOS, KernelSU, and Xiaomi Type-C wired earphones; exact versions, transport, ABI, and mappings remain unmeasured.

## Installation and recovery

There is no release ZIP and no installation procedure yet. Do not install a source archive or treat this documentation as a tested module. The [recovery outline](docs/recovery.md) defines prerequisites for a future prototype; it has not been exercised on the reference phone. No phone access is authorized by this baseline.

## Project documents

- [Contract and decision register](docs/project-contract.md), [Stage 0 review](docs/stage-0-review.md), and [staged backlog](docs/backlog.md)
- [Requirements v1.1](docs/requirements/Wired_Earphone_Module_Agent_Specification.md) and [repository conventions](CONTRIBUTING.md)
- [Architecture](docs/architecture.md), [initial decisions](docs/decisions/0001-initial-design.md), and [threat and failure model](docs/threat-model.md)
- [Enrollment](docs/enrollment.md), [policy and migrations](docs/policy.md), and [root-manager adapters](docs/root-managers.md)
- [Compatibility](docs/compatibility.md), [validation plan](docs/validation-plan.md), and [troubleshooting](docs/troubleshooting.md)
- [Build conventions](docs/build.md), [release process](docs/releasing.md), [privacy](docs/privacy.md), and [research record](docs/research.md)

The approved core is Rust, lifecycle scripts use BusyBox ash, and the offline KernelSU WebUI uses TypeScript compiled to JavaScript with HTML and CSS. Cargo, a pinned Rust toolchain, and the Android NDK will build native Android executables. Tool versions will be pinned before the first build.

Licensed under [MIT](LICENSE). The public repository is [AmanSikarwar/earkey-control](https://github.com/AmanSikarwar/earkey-control). Release publication is deferred until implementation and the specification's acceptance gates are complete. See [SECURITY.md](SECURITY.md) for private reporting status and the [stage review](docs/stage-0-review.md) for actual repository setup.
