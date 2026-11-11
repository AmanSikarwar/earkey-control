# EarKey Control

Configurable blocking of wired earphone button actions.

**Stage 6 is in progress.** The KernelSU alpha provides accessory enrollment, Block all, Custom and pause through a root command interface. Alpha.2 passed basic Center blocking and both boot configurations on the reference ReSukiSU phone. Alpha.4 bundles the offline WebUI with the policy handoff fix and is ready for an authorized upgrade and UI check. [Device results and next test](docs/stage-6-plan.md)

Blocking applies to explicitly enrolled earphone controls. Controls are independently selectable where measured signals permit it. The reference Center signal also handles playback, track changes, the assistant and call actions, so blocking it affects those together. Full preservation of audio, microphone use, connection detection and unrelated input is part of [physical qualification](docs/validation-plan.md#physical-t01t28-matrix).

KernelSU is the first platform, followed by the Magisk and APatch design targets. Android 12 / API 31 and later is the design range. The Redmi Note 10 Pro with PixelOS, the reforge kernel, ReSukiSU and Xiaomi Type-C wired earphones is **Experimental**. The [compatibility matrix](docs/compatibility.md) records the tested alpha builds and pending cases.

## Installation and recovery

For a reference test, build the reviewed installable ZIP using the [pinned recipe](docs/build.md#stage-6-module-package) and follow the [upgrade test procedure](docs/stage-6-plan.md#alpha4-upgrade-and-webui-check--11-november-2026) after authorization. A fresh install defaults to inactive. An enrolled user can enable blocking explicitly through the root commands or, on a validated manager, the WebUI. Public release remains pending.

Use **Pause now** in the WebUI or run `/data/adb/modules/earkey_control/bin/earkey-worker pause` as root. Check `status` for release confirmation. `disable-next-boot` provides a durable inactive setting. See [recovery](docs/recovery.md) for the manager and root paths.

## Project documents

- [Contract and decision register](docs/project-contract.md), [Stage 0 review](docs/stage-0-review.md), and [staged backlog](docs/backlog.md)
- [Requirements v1.1](docs/requirements/Wired_Earphone_Module_Agent_Specification.md) and [repository conventions](CONTRIBUTING.md)
- [Architecture](docs/architecture.md), [initial decisions](docs/decisions/0001-initial-design.md), and [threat and failure model](docs/threat-model.md)
- [Enrollment](docs/enrollment.md), [policy and migrations](docs/policy.md), and [root-manager adapters](docs/root-managers.md)
- [Compatibility](docs/compatibility.md), [validation plan](docs/validation-plan.md), and [troubleshooting](docs/troubleshooting.md)
- [Build conventions](docs/build.md), [release process](docs/releasing.md), [privacy](docs/privacy.md), and [research record](docs/research.md)

The core is Rust, lifecycle scripts use BusyBox ash, and the offline KernelSU WebUI uses TypeScript compiled to JavaScript with HTML and CSS. Cargo, Rust 1.99.1 and Android NDK r28c build the arm64 worker. See the [build and validation commands](docs/build.md).

Licensed under [MIT](LICENSE). The public repository is [AmanSikarwar/earkey-control](https://github.com/AmanSikarwar/earkey-control). Release publication is deferred until implementation and the specification's acceptance gates are complete. See [SECURITY.md](SECURITY.md) for private reporting status and the [stage review](docs/stage-0-review.md) for actual repository setup.
