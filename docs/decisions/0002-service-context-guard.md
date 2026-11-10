# Service-context launcher guard decision

Date: 10 November 2026. Status: **Accepted — Option A, composite identity check; re-baselined 10 November 2026 to the measured reforge kernel profile.** The maintainer selected Option A on 10 November 2026 and selected the re-baseline to the current kernel on the same date. Acceptance applies only to the temporary diagnostic fixture and its metadata-only `self-check`. It does not authorize device access, installation, reboot, input access or any change to the EarKey Control module.

## Context

The temporary `earkey_control_preflight` service hook is intended to prove that the reviewed probe runs from the ReSukiSU services launcher. The guard at the time of this decision required the module ID, `KSU=true`, `BAKASU=true`, userspace `35257`, `KSU_KERNEL_VER_CODE=35240`, UAPI `6` and UID 0 before it checked the probe digest and invoked only `self-check`.

Two authorized runs established that the kernel-version value is not stable during boot:

- On 8 November the services hook saw `32704` at 13.60 s and the later boot-completed hook saw `35240` at 27.78 s. A fresh `ksud debug version` in the services hook also returned `32704`.
- On 9 November the same services hook saw `32704` in samples 1–7, from 92.90 s to 99.27 s, and `35240` at sample 8, 100.34 s. The boot-completed hook at 107.68 s also saw `35240`.

All diagnostic samples reported `full_version=unknown`, flags `0x20` (`PREINIT`), built-in mode, `lkm=false`, `bundled=false`, `late_load=false` and UAPI `6`. No KSU module was loaded, no driver-initialization line or `ksu_change_ksuver` entry appeared in the filtered `dmesg`, and neither `ksu_toolkit` nor `/data/adb/ksu/.manager_version` was present. The writer remains unidentified.

On 10 November 2026 a read-only preflight observed that the phone had booted a different built-in driver: release `4.14.357-reforge-g9b21e3df6c0a`, build `#2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026`, `version=35201`, `full_version=v4.2.0-35201-backslashxx`, flags `0x160`, UAPI `6` and features `0x7`. The manager digest, owner and mode were unchanged and no KSU module was loaded. The previously accepted VantomKernel identity would reject this driver before the metadata check, so the concrete constants needed review rather than silent substitution.

A read-only inspection of the pinned userspace source at `516d3e87` found that module scripts receive `KSU_KERNEL_VER_CODE` from `ksucalls::get_version()` and that the userspace source contains no `CHANGE_KSUVER` invocation. The manager therefore reads the value rather than writing it. This narrows the writer to a module, script, binary or kernel path outside the manager's normal userspace code; it does not identify it.

The value is mutable, so equality on `35240` can reject a correct services context before a writer acts, and it cannot prove that a later value came from the same context. A longer sample window only observes the change. The decision is how to gate the metadata-only launcher check without treating the mutable value as an immutable identity.

## Constraints

- Do not infer service-context success from the boot-completed hook.
- Do not treat the later `35240` value as proof that the services-stage context is correct.
- Do not relax the guard without maintainer agreement and review.
- Do not run `earkey-probe`, open evdev or uinput, grab, inject or change policy.
- Keep host, shell-context, service-hook and physical evidence separate.
- Any new device access needs explicit authorization for that exact scope.
- A metadata-only `self-check` result does not authorize input-capable probes.

## Option A — composite identity invariant for the metadata check

**Selected on 10 November 2026.** Replace the `KSU_KERNEL_VER_CODE=35240` equality with a reviewed composite that does not depend on the mutable version. The concrete values in this section are the original acceptance; the [re-baseline](#re-baseline-to-the-measured-reforge-kernel--10-november-2026) replaces the driver and kernel constants while keeping the option's structure. The accepted gate requires all of:

- the exact diagnostic module ID and staged path;
- UID 0;
- the reviewed `/data/adb/ksud` regular file, owner `0:0`, mode `0755` and SHA-256 `fa0a460a91dc65b4408f9eb63a883342e0b017e84da5e444dee4903ee31d8f50`;
- `KSU_VER_CODE=35257` and `KSU_UAPI_VER=6`;
- `debug info` flags exactly `0x20` (`PREINIT`), `lkm=false`, `bundled=false`, `late_load=false` and `runtime_mode=built-in`;
- the measured Linux kernel release `4.14.357-openela-VantomKernel` and build suffix `#1 SMP PREEMPT Sun Oct 11 07:45:56 UTC 2026` from `/proc/version`;
- no `kernelsu`, `ksu`, `resukisu` or `susfs` entry in `/proc/modules`.

`KSU_KERNEL_VER_CODE` and `full_version` would be recorded as observations, not equality checks. If the kernel identity, manager digest or driver flags do not match, the hook records the mismatch and leaves the probe unexecuted.

**Benefits:** the check does not race the mutable version, it pins the manager and the measured kernel build, and it can run at the services stage without a wait. It is arguably a stronger identity check than a single mutable integer for this reference because the driver is built in and no second driver was observed.

**Costs and residual risk:** this is a scope change, not an exact kernel-code equality check. The kernel build string is not a cryptographic measurement, and `full_version=unknown` means the driver did not supply a compiled-in commit. A kernel with the same build string but different KSU code could pass. The option only covers `self-check`; `probe-uinput` and any input path still need a separately reviewed driver and device gate.

**Authorization status:** the fixture change, host tests and deterministic package are host-side work. Device execution still needs explicit authorization for one temporary module install, one reboot, a bounded record read and one cleanup reboot. No probe input path is involved.

## Option B — bounded post-stabilization version check

Keep the exact `KSU_KERNEL_VER_CODE=35240` requirement, but wait in the services hook until the value is observed twice with the same result, within a reviewed total deadline, before running `self-check`. If it does not stabilize, record the observed values and leave the probe unexecuted.

**Benefits:** retains the existing exact-version guard and requires no new identity model.

**Costs and residual risk:** the services stage may run early and the writer's timing is variable, so this can add avoidable boot delay. A pass only proves that two readings matched at that moment; a later write can still occur, and the check does not establish why the value changed. The residual race and the maximum wait must be stated and tested. A slower boot or a different writer could make the hook skip the probe without proving anything about the context.

**Authorization needed:** the same temporary-module, reboot and cleanup scope as Option A, with the wait bounds reviewed first.

## Option C — bounded read-only writer identification first

Before changing the guard, use separately authorized read-only phone access to search installed scripts and metadata for the writer. The search would cover:

- module `service.sh`, `boot-completed.sh`, `post-fs-data.sh` and similar scripts under `/data/adb/modules/`;
- `/data/adb/service.d/` and `/data/adb/post-fs-data.d/`;
- `/data/adb/ksu/.manager_version` and any documented `toolkit` path;
- literal `32704`, `setver`, `CHANGE_KSUVER` and `ksu_toolkit` references, with only bounded matching context retained.

The search would not execute any script or binary, start the manager, load a module, or reboot. It would also record the module list and file metadata needed to attribute a match. If a writer is found, the maintainer can decide whether to keep the exact guard and account for that writer, or whether the writer can be disabled independently. If nothing is found, the writer remains unknown and Option A or B is still required.

**Benefits:** it is the most evidence-driven next step and may preserve the exact guard. It does not run the probe or change the module set.

**Costs and residual risk:** it needs a new device authorization, may expose private module script text that must be sanitized, and may be inconclusive if the writer is a binary or kernel path. A found writer does not by itself prove that the services-stage value is safe; it only explains the change.

**Authorization needed:** read-only ADB root access to the named paths and bounded output capture. No installation, execution, reboot or policy change.

## Status quo — keep the guard and leave the check Not passed

This option would have kept the exact-version guard unchanged. It is safe but leaves the service-context launcher check blocked and Stage 4 dependent on a future decision. It was not selected.

## Assessment

- Option A is accepted. It is the only option that lets the metadata-only launcher check proceed without waiting on the mutable value. It is scoped explicitly to `self-check` and is not input readiness.
- Option B was not selected. It preserves the exact integer but retains the race and can delay boot.
- Option C remains available as separate evidence work. It may still identify the writer, but it is no longer a prerequisite for this metadata check.
- The status quo no longer applies to host implementation. The device check remains **Not passed** until the composite fixture runs successfully in the services hook under new authorization.

## Decision

**Accepted: Option A — composite identity check for the metadata-only launcher check.** Effective 10 November 2026.

The accepted gate requires all of the following before `earkey-probe self-check` may run from the services hook:

- `KSU=true`, `BAKASU=true`, the exact module ID `earkey_control_preflight`, UID 0, `KSU_VER_CODE=35257` and `KSU_UAPI_VER=6`;
- the reviewed `/data/adb/ksud` regular file, no symlink, owner `0:0`, mode `0755` and SHA-256 `fa0a460a91dc65b4408f9eb63a883342e0b017e84da5e444dee4903ee31d8f50`;
- one bounded `ksud debug info` query whose fields are complete and unique, with `full_version=v4.2.0-35201-backslashxx`, `flags=0x160`, `features=0x7`, `uapi_version=6`, `lkm=false`, `bundled=false`, `late_load=false`, `runtime_mode=built-in` and `pr_build=false`;
- `/proc/version` containing the measured release `4.14.357-reforge-g9b21e3df6c0a` and build suffix `#2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026`;
- no `kernelsu`, `ksu`, `resukisu` or `susfs` entry in `/proc/modules`.

`KSU_KERNEL_VER_CODE` is recorded only as a bounded observation with an unset, empty, numeric or invalid state. The driver's `version` field is recorded only as a bounded numeric observation. Neither is an equality gate.

**Scope:** the metadata-only `self-check` launcher check. This decision does not cover `probe-uinput`, evdev access, grab, injection, passthrough, suppression or any policy change. Those remain separate reviews and separate device authorization.

**Residual risks:** the `/proc/version` release and build string is not cryptographic and can be spoofed through the fork's `CHANGE_SPOOF_UNAME` path; a different kernel with the same measured strings and driver JSON could pass. The driver-reported `version` remains mutable through `ksuver_override`, and `flags` can be replaced through `ksuflags_override` when toolkit support is compiled in; both symbols appeared in `/proc/kallsyms` on this phone. The gate therefore proves the observed state at query time, not provenance. The running kernel's exact source is unidentified, so the meaning of flag bits 6 and 8 is inferred from a related fork rather than proven locally. A future kernel or driver update will fail the gate closed and require another documented re-baseline. The boot-completed hook repeats the identity checks but never runs the probe; a boot-completed pass is not service-context success. An on-device run is still required to show that BusyBox, the manager binary and the services launcher behave as the host simulation assumes.

### Re-baseline to the measured reforge kernel — 10 November 2026

The maintainer selected **Re-baseline to current kernel** on 10 November 2026. This replaces only the measured driver and kernel constants; it does not change the composite structure, weaken any guard, or authorize device execution. The old profile was release `4.14.357-openela-VantomKernel`, build `#1 SMP PREEMPT Sun Oct 11 07:45:56 UTC 2026` and flags `0x20`.

The new measured profile is:

| Field | Measured value |
| --- | --- |
| Kernel release | `4.14.357-reforge-g9b21e3df6c0a` |
| Kernel build | `#2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026` |
| Driver `version` | `35201` (observation only) |
| Driver `full_version` | `v4.2.0-35201-backslashxx` |
| Driver `flags` | `0x160` |
| Driver `features` | `0x7` |
| Driver UAPI | `6` |
| Mode fields | `lkm=false`, `bundled=false`, `late_load=false`, `runtime_mode=built-in`, `pr_build=false` |
| Manager digest | `fa0a460a91dc65b4408f9eb63a883342e0b017e84da5e444dee4903ee31d8f50`, owner `0:0`, mode `0755` |
| Loaded KSU module | none |

`flags=0x160` is consistent with `PREINIT` (`0x20`) plus two bits that the related FinaSU fork names `KSU_GET_INFO_FLAG_HOOK_TRACEPOINT` (`0x40`) and `KSU_GET_INFO_FLAG_HOOK_READY` (`0x100`). FinaSU's `KSU_GET_INFO_FLAG_HOOK_SUSFS_INLINE` (`0x80`) is clear, and the live `/proc/config.gz` advertises `CONFIG_KSU_SUSFS=y` with no inline-hook option. BakaSU's UAPI defines only bits 0–5, so the two high bits are not BakaSU constants. `features=0x7` matches BakaSU's `KSU_FEATURE_MAX` (IDs 0–6), not FinaSU's larger enum. The running kernel's exact source is unidentified; these are sourced related-fork interpretations, not proven local semantics. The gate pins the measured numbers rather than claiming a specific fork.

Sources reviewed 10 November 2026: [BakaSU `uapi/supercall.h` at `516d3e87`](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/uapi/supercall.h), [BakaSU `uapi/feature.h` at `516d3e87`](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/uapi/feature.h), and [FinaSU `uapi/supercall.h` at `adf9a186`](https://github.com/LyraVoid/FinaSU/blob/adf9a18614c099d775a4389e8cc94cc5be31da61/uapi/supercall.h), the commit that added the selectable SUSFS inline-hook backend. The live `/proc/config.gz` and manager query are observations from the phone, not source proof.

The read-only preflight that produced this profile did not install anything, reboot, or run the probe. It also verified that the diagnostic active and staged paths were absent, no module was staged, the manager digest and ownership were unchanged, the six-module inventory contained no diagnostic ID, and the metamodule installer was present but unused. The installer path `/data/adb/modules/magic_mount_rs/metainstall.sh` still needs its own authorization if the normal manager installer delegates to it.

## Implementation status

Host-side implementation is complete for the re-baselined accepted scope:

- fixture: `tests/fixtures/resukisu-service-preflight/`, module version `0.0.9-preflight`, version code 9;
- host suite: `tests/test_service_preflight.py`, 22 passing tests, including the composite gate, flags/features/pr-build failures, mutable-version, driver-version and full-version observations, manager/driver/kernel/module failures, malformed or duplicate fields, installer behavior and deterministic packaging;
- package: `earkey-control-service-preflight-v9.zip`, 362,057 bytes, SHA-256 `1a41ee4def45d74a31a66ec36fdef68472536a43378ec78d054e33251d4a40de`, built twice with identical bytes from probe SHA-256 `bb9469338156678c53eec406903063bfe6c13a46e4377b5c6bfbdd72e24b2f42`.

On 10 November 2026 the authorized v9 run passed the service-context launcher check. See the [v9 result](../prototype-preflight.md#v9-service-context-result--10-november-2026). The text below describes the status before that run.

No new device authorization is granted by this record. The service-context launcher check remains **Not passed** until the maintainer authorizes one temporary install, one reboot, a bounded record read and one cleanup reboot, and that run succeeds. The [preflight](../prototype-preflight.md) records that status.

On 10 November 2026 the authorized v8 run stopped at the services-hook `full_version` guard. The services hook at 25.34 s saw `v3.3.1-32726` and boot-completed at 45.16 s saw `v4.2.0-35201-backslashxx`, so `full_version` is as mutable as the driver `version`. Flags, features, UAPI and mode fields matched in both hooks. See the [result](../prototype-preflight.md#re-baselined-service-context-result--10-november-2026). The maintainer then decided that `full_version` is a recorded observation only, like the driver `version`. The gate keeps flags, features, UAPI, the mode fields, the manager digest and `/proc/version`.
