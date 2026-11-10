# Stage 4 Android preflight

Stage 3 is merged. The standalone capability probe targets arm64 Android at API 31. A separate temporary diagnostic fixture was built to run its metadata-only command from the intended launcher; its first authorized service-hook run stopped at the kernel-version guard before executing the probe (see the [service-context result](#service-context-diagnostic-result--8-november-2026)). The re-baselined v9 run on 10 November passed the launcher check (see the [v9 result](#v9-service-context-result--10-november-2026)). [Build and inspect the probe](build.md#stage-4-android-capability-probe)

## Probe commands

| Command | Operations | Result |
| --- | --- | --- |
| `self-check` | Query its own effective UID and page size, report the built input-event ABI, and read bounded SELinux context and enforcement metadata | `self_check_ok` establishes that this copy loaded and ran in this invocation. Missing SELinux metadata is reported as unavailable |
| `probe-uinput` | Require UID 0, readable enforcing SELinux state and context, and valid private EarKey Control storage with no inactive marker. Check that `/dev/uinput` is the measured character device 10:223. Open once, verify the opened node and close it | `uinput_open_ok` establishes open permission for this invocation. No data or ioctls are sent to the device |

The inactive guard uses the existing root-owned 0700 storage directory and a regular, single-link root-owned 0600 `stay-inactive` marker. Any invalid marker or unavailable storage stops the open. The probe neither creates nor clears storage. It accepts only the commands above and has no input path or accessory argument.

## Order of physical checks

1. After separate authorization, copy the reviewed ELF to a temporary phone path and run only `self-check` from the existing root shell, with a bounded command window. Verify the on-device digest before executing. Capture only the probe's own output, then remove the temporary copy and verify removal. The expected reference outputs include AArch64, 4096-byte pages, input-event size 24 and type offset 16. Log the actual exit status and SELinux metadata. This is a load check in the shell context.
2. Use a separately reviewed and authorized temporary service hook to verify the intended launcher. The exact ReSukiSU userspace source routes the once-per-boot services event through active modules’ `service.sh` hooks and BusyBox ash. The global services entry can also run unrelated scripts. A shell, installer hook or manager action can provide its own observation. Record effective UID, SELinux label and enforcing state for the service invocation itself. [Exact fork lifecycle source](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/init_event.rs#L143-L191), [KernelSU module lifecycle](https://kernelsu.org/guide/module.html)
3. Run the open-only `probe-uinput` in the intended service context after explicitly provisioning and validating the private guard. Measure denial or success as observed. Next implement and review a separately scoped evdev descriptor probe that uses verified accessory enrollment and revalidates the opened collection. Stop if the service lacks access. Any policy change needs a narrow justification and separate review.
4. Before any grab, review the temporary worker and independent supervisor. Prove the no-grab pause and deadline paths first. Measure proxy readiness and mapping before enabling a supervised session. Verify neutral acquisition, single repeat ownership, independent pause, bounded release and the full preservation matrix. [Architecture](architecture.md), [Recovery requirements](recovery.md), [Validation](validation-plan.md)

## Reference shell-context self-check — 8 November 2026

The first authorized bounded check ran on the reference phone from the existing root shell. The reviewed arm64 ELF from review head `ea71cfa` (SHA-256 `bb9469338156678c53eec406903063bfe6c13a46e4377b5c6bfbdd72e24b2f42`) was copied to a private temporary directory. The on-device digest matched before execution, and only `self-check` ran. It exited 0 and reported:

```text
arch=aarch64
page_size=4096
input_event_size=24
input_event_type_offset=16
effective_uid=0
selinux_context=u:r:ksu:s0
selinux_enforcing=1
probe=self_check_ok
```

The temporary copy was removed and its absence verified; no probe process was observed. An intermediate host orchestration attempt exited on a shell variable error after the probe returned and left its temporary directory behind; that directory was removed and verified. This record establishes only that the reviewed copy loaded and ran in the root shell context. It does not establish service-context execution, the `probe-uinput` guard, evdev access, grab, proxy behavior, passthrough, audio or microphone preservation, or Stage 4 exit.

## Temporary service diagnostic

The diagnostic has its own module ID, `earkey_control_preflight`. Its ZIP contains the exact verified ELF, a digest file, a boot-mode installer check, `service.sh`, `skip_mount`, module metadata and the license. The manager's native ZIP installer handles staging and normal boot activation. The installer guards boot mode, the measured `KSU`/`BAKASU` userspace, userspace 35257, UAPI 6, arm64 and Android API 37. It does not check the mutable kernel-version value. The service hook gates on the accepted composite identity instead: the exact module ID, UID 0, the reviewed manager file and digest, one bounded `ksud debug info` result with the measured `full_version`, flags and features, the measured kernel release/build from `/proc/version`, and no loaded KSU module. The driver `version` and `KSU_KERNEL_VER_CODE` are observations only. A matching installer or root-shell query does not establish that the service launcher sees the same identity.

The pinned fork's service launcher derives `KSU_KERNEL_VER_CODE` from its kernel information query, which caches the result within the process. A separate shell query is a different invocation. The earlier exact-version guard was not weakened on the basis of a shell result; the accepted composite gate replaces it only for the metadata check after the temporal write was established. Do not use a separate query to bypass any remaining device gate. This is informed by the [launcher](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/module/mod.rs) and [query implementation](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/ksucalls.rs), researched 8 November 2026. The pinned source alone does not prove the running kernel's implementation.

The earlier v5/v6 fixture compared a kernel-version failure with one fresh `ksud debug version` invocation in the service context. It first required UID 0, the expected UAPI environment, a root-owned mode-0755 regular nonsymlink manager binary, and the SHA-256 of the arm64 executable in the [official packaged build](https://github.com/Baka-SU/BakaSU/actions/runs/41207077584) for the pinned revision. The packaged binary is distinct from the raw build artifact. Both the digest check and query had five-second deadlines, and only bounded ASCII-digit output was recorded. That diagnostic showed the value changing within one hook invocation; the accepted fixture no longer runs `debug version`. It verifies the same manager file and digest, then runs one bounded `ksud debug info` query for the composite fields. The pinned [debug command](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/cli.rs#L821-L823) shows both queries; driver acquisition can include the fork's special syscall and a kernel information ioctl.

The pinned fork also invokes a module's `boot-completed.sh` from a later [boot-completed stage](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/init_event.rs#L193-L208). This fixture uses that separate, one-shot hook to repeat the composite identity checks and the bounded observation. The hook never runs the probe. Each hook records a boot-uptime marker at context capture, its own shell's bounded SELinux label and mount and UTS namespace identifiers, or marks them unavailable. The stages launch without waiting, so compare the markers instead of assuming execution order. The hook records are separate processes; even matching recorded context fields do not establish complete context equivalence. The `guard_exit` field in the later result reports composite guard success, not service-context success.

The service reserves one attempt per installation in its own private `.preflight` directory and records `hook=entered` before checking the launcher guards. A rejected environment guard records its name in `guard_failure`. The mutable kernel-version value is recorded only as unset, empty, numeric or invalid, with ASCII digits up to ten characters; `full_version` is recorded as an observation. The composite gate reports `manager_identity`, `driver_identity` with an optional `driver_reason`, `kernel_identity` and `modules_identity`, plus bounded `debug info` fields. Other environment values are not recorded. After all identity checks pass, the hook checks the ELF SHA-256 and calls only `self-check`. The hash check and probe each have a five-second kill deadline. The hook writes the artifact digest, probe output and exit code to `.preflight/result` through a private temporary file. An existing attempt, disable or remove marker inhibits execution. A nonzero probe exit is evidence to retain. Inspect `.preflight/result.tmp` if the final result is absent. If both are absent after a bounded wait, the launcher may have skipped the hook or the hook may have failed before creating its private directory.

The host fixture checks use `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_service_preflight.py -v`. The package check requires the locally built reviewed ELF. The tests simulate the launcher with dash and do not establish on-device BusyBox behavior.

Build the ZIP with the verified preflight ELF and Python 3.15.2. The packager checks the ELF digest, writes a fixed archive inventory and refuses to overwrite an existing output.

```sh
python3 tools/package-service-probe.py \
  target/stage4/1506082/earkey-probe-arm64-api31 \
  target/stage4/earkey-control-service-preflight.zip
```

After explicit authorization for installation and both reboots, apply this sequence:

1. Check the exact reference phone, live ReSukiSU versions, normal completed boot, a usable existing root shell, and absence of the diagnostic ID in both `modules` and `modules_update`. Check for pending manager operations and active metamodule delegation before staging. Stop if another pending install or removal could apply on reboot. Review and separately authorize any delegated installer or removal hooks before using them.
2. Stage the ZIP at a temporary private phone path and check its full digest. Use the measured fork's `/data/adb/ksud module install <verified-zip>` command. Verify the exact diagnostic ID is staged and remove the temporary upload. The manager performs its normal installation and preinit bookkeeping. [Fork installer and module commands](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/module/mod.rs)
3. Reboot normally and wait for the existing ADB root route. Read the diagnostic's `.preflight/result` and `.preflight/context`, and its `.boot-preflight/result` and `.boot-preflight/context`. Check the corresponding `.tmp` files if an expected record is absent. A successful service test records the reviewed digest, `launcher=resukisu_service`, `probe=self_check_ok`, `service_probe_exit=0`, effective UID, SELinux context, enforcing state and ABI. Compare hook context fields with each other and with the shell observation. Retain the sanitized record privately.
4. Mark only `earkey_control_preflight` for removal with `/data/adb/ksud module uninstall earkey_control_preflight` and verify its `remove` marker. The exact fork skips removed modules and prunes them during the next normal boot. Reboot for cleanup and verify the active and staged paths, including the private result, are gone. [Fork boot and pruning sequence](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/init_event.rs)

Use the diagnostic result to establish the service launcher context. Device-node permissions, independent worker supervision and physical passthrough require their own checks.

## Service-context diagnostic result — 8 November 2026

The services hook ran from the temporary module after one normal reboot. Installation used the separately authorized `magic_mount_rs/metainstall.sh` path. Its legacy-partition loop emitted same-file `cp` errors for `/system/{vendor,product,system_ext}` and the corresponding root paths; the staged tree contained no partition payload. The manager staged the module at `/data/adb/modules_update/earkey_control_preflight`, created the normal `update` marker, and activated it on the next boot. The reviewed package was `earkey-control-service-preflight-v6.zip`, 353,784 bytes, SHA-256 `1c74e236dfca9a03bf3c9ca890f865600a9518f0022fb1470f683d4a6f7c8eaa`; the on-device digest matched before installation and the embedded probe matched `bb9469338156678c53eec406903063bfe6c13a46e4377b5c6bfbdd72e24b2f42`.

The services hook entered at `hook_uptime_seconds=13.60` and stopped at the kernel-version guard. Its sanitized result was:

```text
hook=entered
guard_failure=kernel_version
kernel_version_state=numeric
kernel_version_value=32704
fresh_query=numeric
fresh_query_value=32704
service_probe_exit=1
```

`service_probe_exit=1` is the hook status after the failed guard; `earkey-probe self-check` did not run. A fresh `ksud debug version` invocation from the same hook also returned `32704`, so the failure was not limited to the manager's inherited environment value. The hook context was:

```text
hook_uptime_seconds=13.60
selinux_context=u:r:ksu:s0
mnt_namespace=4026533853
uts_namespace=4026531838
fd_scan=complete
driver_cloexec_yes=0
driver_cloexec_no=0
driver_cloexec_unknown=0
driver_su_cloexec_yes=0
driver_su_cloexec_no=0
driver_su_cloexec_unknown=0
```

The later boot-completed hook entered at `hook_uptime_seconds=27.78` and reported:

```text
hook=entered
kernel_version_state=numeric
kernel_version_value=35240
fresh_query=numeric
fresh_query_value=35240
guard_exit=0
```

Its context matched the services hook in the recorded SELinux label, mount namespace and UTS namespace, and its descriptor scan was also complete with zero counted KSU driver descriptors. `guard_exit=0` reports guard success; this hook does not invoke the probe. The two hook records are separate processes, and the matching fields do not establish complete context equivalence.

The service-context launcher check is **Not passed**. The services hook did not execute the probe, and the kernel-version values differ by stage: `32704` at 13.60 s and `35240` at 27.78 s. The authorized follow-up found that the value changes during boot: the same services hook observed `32704` through 99.27 s and `35240` at 100.34 s, so the difference is temporal rather than context-specific. The writer remains unidentified. The maintainer accepted the [composite identity check](decisions/0002-service-context-guard.md) on 10 November 2026 for the metadata-only `self-check`; it replaces the mutable equality with a verified manager, driver, kernel and module identity. The host fixture and a deterministic package are prepared. The phone later booted a different built-in kernel, and the maintainer re-baselined the composite constants on 10 November 2026; no re-baselined run has been authorized or executed. A boot-completed pass is not substituted for the services hook. The retained private summary records 13.64 s and 26.81 s for the same two hooks; the values above differ slightly by measurement point. Treat the precise entry times as approximate; the stage ordering and guard outcomes are unchanged. The hook context records `selinux_context` but not enforcing state; the separate shell-context observation reported Enforcing, but that is not a service-hook record. The complete zero-count descriptor scan describes only the hook shell, not the manager process. No service-context probe, evdev access, uinput operation, grab, passthrough, suppression, audio, microphone or physical behavior is established.

The bounded raw records are retained privately. The temporary upload was removed after installation. `/data/adb/ksud module uninstall earkey_control_preflight` created the `remove` marker; after the authorized cleanup reboot, the active and staged module paths, both private record directories, the temporary upload and all pending markers were absent. The module set returned to the original six and the `magic_mount_rs` metamodule symlink was unchanged. No phone serial or temporary ADB detail is recorded here.

### Kernel-version source note

The pinned kernel's `GET_INFO` handler starts from `KERNEL_SU_VERSION` but replaces it with `ksuver_override` when that value is non-zero. The fork's `sys_reboot` extension accepts `CHANGE_KSUVER` from any UID 0 process; the pinned kernel does not tie the override to the manager or to a specific service stage. [dispatch.c](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/kernel/supercall/dispatch.c#L71-L98), [supercall.c](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/kernel/supercall/supercall.c#L139-L150). The observed `32704` could therefore be a transient override rather than the kernel build's own version.

A public unattended setter exists: `ksu_toolkit` implements `--setver` with `CHANGE_KSUVER`, and its `service.sh` applies `/data/adb/ksu/.manager_version` at service stage. [toolkit.c](https://github.com/backslashxx/ksu_toolkit/blob/4ae075ceb3d6a07f683965aa68bca4126a9be4de/src/toolkit.c#L244-L245), [service.sh](https://github.com/backslashxx/ksu_toolkit/blob/4ae075ceb3d6a07f683965aa68bca4126a9be4de/module/service.sh#L22-L24). The pinned kernel assigns `ksuver_override` only in the `CHANGE_KSUVER` branch and has no reset path, so a toolkit application persists until reboot or driver unload. [dispatch.c](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/kernel/supercall/dispatch.c#L1208-L1215). A change from `32704` to `35240` therefore needs a second writer or a different driver; the toolkit's service hook alone cannot produce it. The recorded module set did not include `ksu_toolkit`, and no setter or `ksu_change_ksuver` log was observed. It remains a candidate for the follow-up, not evidence that it ran on this device. The records do not identify the setter, and no process or module was observed making the call. The follow-up must not assume the services-stage value is a stable kernel property; it must establish the source of the value before the guard is changed or bypassed.

The Kbuild formula is `KSU_VERSION = 30000 + git rev-list --count HEAD + 700`. At pinned commit `516d3e87`, the count is 4557, so the userspace build is 35257, matching the recorded `KSU_VER_CODE`; the kernel-stage 35240 corresponds to a count of 4540, 17 commits earlier. The recorded 32704 corresponds to a count of 2004, far outside this commit history. That makes 32704 unlikely to be this kernel build's own version, but it does not prove an override or establish which driver answered either query. The follow-up must distinguish a `CHANGE_KSUVER` override, a second or earlier KSU driver, and a different query path; it must not assume any one explanation.

### Kernel-version follow-up plan (authorized and run)

The plan below was executed on 9 November 2026; the result follows it. The maintainer authorized this diagnostic on 9 November 2026. It may install one temporary module, reboot once, read bounded records, mark only that module for removal, and reboot once for cleanup. It does not install EarKey Control, execute `earkey-probe`, loosen the guard, open evdev or uinput, grab, inject, or change policy.

The diagnostic has its own module ID, `earkey_control_kver`. The ZIP contains `module.prop`, `customize.sh`, `service.sh`, `boot-completed.sh`, `skip_mount` and the license; it contains no ELF, no probe digest file and no EarKey Control code. The installer guards for boot mode, the measured `KSU`/`BAKASU` userspace, userspace 35257, UAPI 6, arm64, Android API 37 and the exact staged path. It deliberately does not guard on `KSU_KERNEL_VER_CODE`, because the reported value is the subject of the diagnostic. The hooks also require the expected module ID, userspace and UAPI, but not the kernel-version value, before querying.

The services hook records the environment values (`KSU`, `BAKASU`, `KSU_MODULE`, `KSU_VER_CODE`, `KSU_KERNEL_VER_CODE`, `KSU_UAPI_VER`, `KSU_RUNTIME_MODE`), then verifies `/data/adb/ksud` owner, mode and SHA-256 and runs it with `debug info` and `debug version` at entry and in a bounded series of no more than eight samples over no more than ten seconds. It captures only the `debug info` fields `version`, `full_version`, `flags`, `uapi_version`, `lkm`, `bundled`, `late_load`, `runtime_mode` and `pr_build`, not raw output. `full_version` is compiled into the driver and is not replaced by the override, so it can identify the kernel build tag and commit independently of the mutable `version`; `unknown` is recorded if the build did not embed it. The boot-completed hook records the same snapshot once, without a series. Both hooks record the SELinux label, UID and mount and UTS namespaces, and verify the same fixed `/data/adb/ksud` owner, mode and SHA-256 before execution and record a classification on mismatch. They also record:

- structured `dmesg` extracts from the most recent 400 matching lines: the driver initialization version, compiled-in `full_version`, ABI, work mode and UAPI values, and the ordered `ksu_change_ksuver` values, at most 16 with a count and overflow flag, or `unavailable` if access is denied. The record contains only these fields, not raw log lines;
- filtered `/proc/modules` entries for `kernelsu`, `ksu`, `resukisu` or `susfs`, names and sizes only, or `unavailable` if access is denied;
- bounded `/proc/config.gz` values for `CONFIG_KSU`, `CONFIG_KSU_TOOLKIT_SUPPORT`, `CONFIG_KSU_SUSFS` and `CONFIG_KSU_MULTI_MANAGER_SUPPORT`, recorded as `y`, `m`, `n`, absent or unavailable;
- metadata and the first line of `/data/adb/ksu/.manager_version` if it exists, validated as at most ten decimal digits and never executed;
- module directory names and whether `/data/adb/modules/ksu_toolkit` or a `toolkit` binary is present at a listed path, without running either.

The fixture is `tests/fixtures/resukisu-kernel-version/`; `tools/package-kernel-version-probe.py` builds the deterministic ZIP. The host checks use `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p 'test_kernel_version_preflight.py' -v`. They simulate the hooks with dash and do not establish on-device BusyBox behavior.

The plan distinguishes the explanations. `full_version`, the driver flags, the init log line, `/proc/config.gz` and `/proc/modules` establish which driver answered and whether a second driver exists; a reported commit whose Kbuild count is 4540 makes `35240` that build's own version, while a reported commit of 4557 makes it an override. `ksu_change_ksuver` values that bracket the observed values establish a writer, and the `.manager_version` and module-presence records identify the candidate setter without executing it. Differing `lkm` or `late_load` flags, or a module load between stages, establish driver coexistence. If none of these appears, the query path remains unresolved and the guard stays unchanged. No result from this plan counts as service-context launcher success; the probe still has to run from the intended services hook with the unchanged guard.

### Kernel-version follow-up result

Preflight matched the reference: UID 0, Android API 37, `sweetin`, build `CP2A.260605.016` / `1793394994`, arm64, the expected `/data/adb/ksud` digest `fa0a460a91dc65b4408f9eb63a883342e0b017e84da5e444dee4903ee31d8f50`, six original modules, no pending remove marker, `/data/adb/modules_update` root-owned `0700`, and both diagnostic paths absent. The deterministic ZIP was 31,183 bytes with SHA-256 `48f8b8ba2bc82acfdc9bb0d02b43c48d55910d3b845e95070a338f0afe2991b3`. Its on-device digest matched before installation. `ksud module install` exited 0 with only the expected metamodule same-file `cp` errors. The staged `service.sh`, `boot-completed.sh` and `module.prop` hashes matched the fixture, and the installer removed `customize.sh` after sourcing it. Before the reboot the active path contained only `module.prop` and an `update` marker, so the hooks could not run before activation. The temporary upload was removed and verified absent.

After one diagnostic reboot, the services hook sampled from 92.90 s to 100.34 s uptime and the boot-completed hook sampled at 107.68 s. The services record was 3,599 bytes with SHA-256 `b98d0011b22647d67bb5484bb736b38e7cefd80985c0813134ef8cd57030bad0`; the boot record was 1,422 bytes with SHA-256 `e9ec49ca6aa35243c71f95b14aac423433abcf16b0e5a72025b9c971a005852f`. Both were mode `0600` in a `0700` directory and remain private.

The services hook's inherited `KSU_KERNEL_VER_CODE` was `32704`, matching the first seven fresh `debug info` and `debug version` queries. Samples 1–7, from 92.90 s to 99.27 s, reported `32704`. Sample 8 at 100.34 s reported `35240`. The boot-completed hook at 107.68 s also reported `35240`. The change therefore happened inside one service-hook invocation, between samples, before the boot-completed hook. This rules out a service-context-only query failure: the same fixed manager binary, SELinux label (`u:r:ksu:s0`), UID, mount namespace and UTS namespace produced both values. The remaining explanation is a time-varying write, not a context-specific read; the timing is consistent with a boot-completed writer, but the writer is not identified.

The records do not identify the writer:

- every sample reported `full_version=unknown`, so the driver did not supply a compiled-in build identity;
- every sample reported flags `0x20` (`PREINIT`), with `lkm=false`, `bundled=false`, `late_load=false` and `runtime_mode=built-in`; there was no second loaded driver;
- `/proc/modules` contained no `kernelsu`, `ksu`, `resukisu` or `susfs` entry;
- the filtered `dmesg` extract was available but contained no driver-initialization line and no `ksu_change_ksuver` value; the record does not contain raw log lines, so an empty or rotated buffer cannot be distinguished from a writer that did not log;
- `/proc/config.gz` reported `CONFIG_KSU=y` and `CONFIG_KSU_SUSFS=y`, but did not contain `CONFIG_KSU_TOOLKIT_SUPPORT` or `CONFIG_KSU_MULTI_MANAGER_SUPPORT`; the running config does not advertise those options, which is not proof that every build lacks the code;
- `/data/adb/ksu/.manager_version` was absent, and `/data/adb/modules/ksu_toolkit` and both of its `toolkit` paths were absent;
- the module set was the original six plus `earkey_control_kver`; no unexpected module was present.

The documented `ksu_toolkit` path was inactive on this boot. The value changed before the boot-completed hook, but no observed module or log line attributes the change to a specific writer. The writer remains unresolved. A longer sampling window alone would only observe the later value; it would not establish an immutable identity. The maintainer subsequently accepted the composite identity check for the metadata-only launcher test. The service-context launcher check remains **Not passed**, and no probe ran; no evdev, uinput, grab, injection, policy or physical behavior was exercised.

`ksud module uninstall earkey_control_kver` exited 0 and created the `remove` marker on the active path. After the authorized cleanup reboot, the active and staged paths, both record directories, the temporary upload and all pending remove markers were absent. The module set returned to the original six, the `magic_mount_rs` metamodule symlink was unchanged, and the manager digest and reported `35240` value were unchanged. No phone serial or temporary ADB detail is recorded here.

## Composite identity gate — accepted 10 November 2026

The maintainer accepted the composite identity check in [decision record 0002](decisions/0002-service-context-guard.md). The fixture now treats `KSU_KERNEL_VER_CODE` as a bounded observation rather than an equality gate. Before `self-check`, the services hook requires the exact module ID, UID 0, `KSU`/`BAKASU` and userspace 35257/UAPI 6, the reviewed `/data/adb/ksud` regular file with owner `0:0`, mode `0755` and SHA-256 `fa0a460a91dc65b4408f9eb63a883342e0b017e84da5e444dee4903ee31d8f50`, one complete and unique `debug info` result with the measured driver identity, the measured kernel release/build from `/proc/version`, and no `kernelsu`, `ksu`, `resukisu` or `susfs` entry in `/proc/modules`. The boot-completed hook repeats the identity checks but never runs the probe.

The host suite is `tests/test_service_preflight.py` with 21 passing tests. The fixture module version is `0.0.8-preflight` (version code 8). The deterministic package is `earkey-control-service-preflight-v8.zip`, 362,255 bytes, SHA-256 `3643317502ec07f7e140da6dff299c8395ddf334703acbb25646beae11d9c929`; it was built twice with identical bytes from probe SHA-256 `bb9469338156678c53eec406903063bfe6c13a46e4377b5c6bfbdd72e24b2f42`.

This acceptance covers only the metadata-only launcher check. It is not input readiness. The `/proc/version` string is not cryptographic and can be spoofed through the fork's uname path. The driver `version` remains mutable and the flags can be replaced through a toolkit override; the gate proves the observed state at query time, not provenance. The service-context launcher check remains **Not passed** because the authorized 10 November v8 run stopped at the services-hook `full_version` guard before the probe ran. The next device step needs explicit authorization for one temporary install, one reboot, a bounded record read and one cleanup reboot. `probe-uinput`, evdev, uinput, grab, injection, passthrough, suppression and policy changes remain outside this scope.

## Re-baseline to the measured reforge kernel — 10 November 2026

A read-only preflight on 10 November found that the reference phone had booted a different built-in driver from the one pinned by the original composite acceptance. It did not install anything, reboot, run the probe, or change policy. The maintainer reports installing the new kernel deliberately, which explains the changed release, build and driver identity; it does not explain the earlier in-boot `KSU_KERNEL_VER_CODE` change, whose writer remains unidentified. The earlier shell-context self-check ran on the previous kernel and is not revalidated. The previously accepted VantomKernel identity would have rejected the new driver, so the maintainer selected **Re-baseline to current kernel** on 10 November 2026. The concrete constants changed; the composite structure and all authorization limits did not.

| Field | Previously accepted | Current measured |
| --- | --- | --- |
| Kernel release | `4.14.357-openela-VantomKernel` | `4.14.357-reforge-g9b21e3df6c0a` |
| Kernel build | `#1 SMP PREEMPT Sun Oct 11 07:45:56 UTC 2026` | `#2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026` |
| Driver `version` | observation | `35201` (observation) |
| Driver `full_version` | observation (`unknown`) | `v4.2.0-35201-backslashxx` (exact gate) |
| Driver `flags` | `0x20` | `0x160` |
| Driver `features` | not recorded | `0x7` |
| Driver UAPI | `6` | `6` |
| Mode fields | built-in, no LKM/bundled/late load | built-in, no LKM/bundled/late load, `pr_build=false` |
| Manager | unchanged | unchanged, digest `fa0a460a91dc65b4408f9eb63a883342e0b017e84da5e444dee4903ee31d8f50`, owner `0:0`, mode `0755` |
| Loaded KSU module | none | none |

The v8 service hook required the exact measured `full_version`, `flags=0x160`, `features=0x7`, UAPI 6, built-in mode and `pr_build=false`, together with the new release/build string. The driver `version` remains an observation, as does `KSU_KERNEL_VER_CODE`.

`flags=0x160` is consistent with `PREINIT` (`0x20`) plus FinaSU's `KSU_GET_INFO_FLAG_HOOK_TRACEPOINT` (`0x40`) and `KSU_GET_INFO_FLAG_HOOK_READY` (`0x100`). The `0x80` inline-SUSFS-hook bit is clear, and the live `/proc/config.gz` advertises `CONFIG_KSU_SUSFS=y` with no inline-hook option. BakaSU's UAPI defines only bits 0–5, so the high bits are not BakaSU constants. `features=0x7` matches BakaSU's `KSU_FEATURE_MAX` (IDs 0–6), not FinaSU's larger enum. The running kernel's exact source is unidentified, so this is a sourced related-fork interpretation, not proven local semantics; the gate pins the measured numbers rather than claiming a specific fork. Sources reviewed 10 November 2026: [BakaSU `uapi/supercall.h` at `516d3e87`](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/uapi/supercall.h), [BakaSU `uapi/feature.h` at `516d3e87`](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/uapi/feature.h), and [FinaSU `uapi/supercall.h` at `adf9a186`](https://github.com/LyraVoid/FinaSU/blob/adf9a18614c099d775a4389e8cc94cc5be31da61/uapi/supercall.h).

The read-only preflight also verified that the diagnostic active and staged paths were absent, no module was staged, the manager digest and ownership were unchanged, the six-module inventory contained no diagnostic ID, `/data/adb/ksu/.manager_version` and the `ksu_toolkit` paths were absent, and `/data/adb/modules/magic_mount_rs/metainstall.sh` was present but unused. The installer path needs separate authorization if the normal manager installer delegates to it. The kernel release, build, driver and flag values are measured observations; the interpretation of the high flag bits is informed inference. The version remains mutable through `ksuver_override`, and the flags can be replaced through `ksuflags_override` when toolkit support is compiled in; both symbols appeared in `/proc/kallsyms`. The kernel string can be spoofed through `CHANGE_SPOOF_UNAME`. None of these observations establishes service-context success.

The service-context launcher check remains **Not passed** because the authorized 10 November v8 run stopped at the services-hook `full_version` guard before the probe ran. A new device authorization is required for one temporary install, one reboot, a bounded record read and one cleanup reboot.

## Re-baselined service-context result — 10 November 2026

The maintainer authorized one temporary install of `earkey-control-service-preflight-v8.zip`, one reboot, a bounded record read and one cleanup reboot. Pre-checks matched the re-baselined profile in a root shell: `/proc/version` reported the reforge release/build, and `debug info` reported `full_version=v4.2.0-35201-backslashxx`, flags `0x160`, features `0x7`, UAPI 6, built-in mode and `pr_build=false`. The manager digest and ownership were unchanged, no module was pending and no diagnostic path existed. `uname -r` in the same shell reported `4.14.357-openela-VantomKernel`, consistent with uname spoofing; the gate reads `/proc/version`. The delegated `magic_mount_rs/metainstall.sh` and `metauninstall.sh` matched the previously reviewed SHA-256 values `c64ccfe7397a80fba8c6269ee854b854b68cd329cfd0d385574df0a524c7ef02` and `1cce72f2b303010a17695b6ccecbc066e7a8e4f7d2b0a44b0ec3ea6b0fe917d0`. The ZIP has no `system/` tree.

The on-device ZIP digest matched `3643317502ec07f7e140da6dff299c8395ddf334703acbb25646beae11d9c929`. `ksud module install` staged the module through the metamodule installer with the same harmless same-file `cp` messages seen for v6, and the staged probe matched `bb9469338156678c53eec406903063bfe6c13a46e4377b5c6bfbdd72e24b2f42`. The upload was removed.

| Hook | Uptime | Driver `version` | Driver `full_version` | Other `debug info` fields | Result |
| --- | --- | --- | --- | --- | --- |
| services | 25.34 s | `32726` | `v3.3.1-32726` | flags `0x160`, features `0x7`, UAPI 6, built-in, no LKM/bundled/late load, `pr_build=false` | `driver_identity=mismatch`, `driver_reason=full_version`, `guard_failure=driver_identity`; probe not run |
| boot-completed | 45.16 s | `35201` | `v4.2.0-35201-backslashxx` | same | composite guard verified, `guard_exit=0` |

Manager, kernel and module identities were verified in both hooks. Both hooks recorded `u:r:ksu:s0` with the same mount and UTS namespace identifiers, and neither saw a driver descriptor. The service-context launcher check remains **Not passed**.

`full_version` is therefore mutable during boot together with the driver `version`, so pinning it reproduces the original temporal failure. The flags, features, UAPI and mode fields matched in both hooks. The pinned userspace `ksud` source contains no version writer, so the writer is still unidentified; it may be in-kernel or the manager app. That is an unverified hypothesis.

The module was marked for removal with `ksud module uninstall`, and the `remove` marker was verified. After the cleanup reboot, the active and staged paths, records and upload were gone. The original six modules, the `magic_mount_rs` metamodule link, the manager digest and Enforcing mode remained. No `probe-uinput`, evdev, uinput, grab, passthrough, suppression, audio or microphone behavior was exercised. Raw records remain private.

## Full-version observation — 10 November 2026

After the v8 result, the maintainer decided that the driver `full_version` is a recorded observation only, like the driver `version` and `KSU_KERNEL_VER_CODE`. The services hook still requires the manager file and digest, one complete `debug info` result with `flags=0x160`, `features=0x7`, UAPI 6, built-in mode, no LKM/bundled/late load and `pr_build=false`, the reforge release/build in `/proc/version`, and no loaded KSU module. A malformed or duplicate `full_version` line still makes the info result invalid.

The fixture is `0.0.9-preflight` (version code 9). The host suite has 22 passing tests. `earkey-control-service-preflight-v9.zip` is 362,057 bytes with SHA-256 `1a41ee4def45d74a31a66ec36fdef68472536a43378ec78d054e33251d4a40de`; two builds were byte-identical, from probe SHA-256 `bb9469338156678c53eec406903063bfe6c13a46e4377b5c6bfbdd72e24b2f42`. The v8 run showed that the remaining pinned fields matched in the services hook, but that is not a v9 result. The [v9 result](#v9-service-context-result--10-november-2026) passed the service-context launcher check.

## v9 service-context result — 10 November 2026

The maintainer authorized the same v9 scope used for v8: one temporary install, one reboot, a bounded record read and one cleanup reboot. Pre-checks matched the re-baselined profile. The manager and both metamodule hook digests were unchanged, no module operation was pending, and no diagnostic path existed. The on-device ZIP digest matched `1a41ee4def45d74a31a66ec36fdef68472536a43378ec78d054e33251d4a40de`. `ksud module install` staged `0.0.9-preflight`, the staged probe matched `bb9469338156678c53eec406903063bfe6c13a46e4377b5c6bfbdd72e24b2f42`, and the upload was removed.

The service-context launcher check **Passed**.

| Hook | Uptime | Driver `version` / `full_version` | Composite identity | Result |
| --- | --- | --- | --- | --- |
| services | 13.41 s | `32726` / `v3.3.1-32726` (observations) | manager, driver, kernel and modules verified | `launcher=resukisu_service`, `probe=self_check_ok`, `service_probe_exit=0` |
| boot-completed | 27.14 s | `35201` / `v4.2.0-35201-backslashxx` (observations) | all verified | `guard_exit=0`; probe not run by design |

The probe reported artifact SHA-256 `bb9469338156678c53eec406903063bfe6c13a46e4377b5c6bfbdd72e24b2f42`, `arch=aarch64`, `page_size=4096`, `input_event_size=24`, `input_event_type_offset=16`, `effective_uid=0`, `selinux_context=u:r:ksu:s0` and `selinux_enforcing=1`. Both hooks recorded the same SELinux label and mount and UTS namespace identifiers. Neither saw a driver descriptor. The driver identity again changed between hooks, and its writer remains unidentified.

This establishes only that the reviewed probe loads and runs from the ReSukiSU services launcher in `u:r:ksu:s0` under enforcing SELinux on this kernel. It does not test `/dev/uinput` access, evdev, grabs, the EarKey Control module, passthrough or suppression. The `input_event` layout matches the probe's built ABI, but that is a self-report, not an evdev read.

The module was marked for removal and the `remove` marker was verified. After the cleanup reboot, the active and staged paths and the upload were gone. The original six modules, the `magic_mount_rs` metamodule link, the manager digest and Enforcing mode remained. Raw records remain private.

## Service-context uinput open plan — 10 November 2026

The next check is step 3 of the physical order: run `probe-uinput` from the ReSukiSU services hook. After the composite gate and the probe digest check pass, the v10 fixture runs `probe-uinput` instead of `self-check`. The probe prints the same self-check fields. It then requires UID 0, enforcing SELinux and a readable context, a valid root-owned 0700 `/data/adb/earkey_control` directory, and no `stay-inactive` marker. It checks that `/dev/uinput` is character device 10:223, opens it once with `O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK | O_NOCTTY`, re-checks the opened node and closes it. It sends no data and no ioctls, so no virtual input device is created. A denial is recorded as the failing stage with its errno. The boot-completed hook still never runs the probe.

The fixture is `0.0.10-preflight` (version code 10). The host suite has 23 passing tests, including a recorded denial. `earkey-control-service-preflight-v10.zip` is 362,061 bytes with SHA-256 `3e59f8da9582b40ce6c43b4d0fa83f8aa20cb7f8c8c4f3bac31d85bb5fb07997`; two builds were byte-identical. It embeds the same reviewed probe, SHA-256 `bb9469338156678c53eec406903063bfe6c13a46e4377b5c6bfbdd72e24b2f42`.

The probe neither creates nor clears storage, so the run explicitly provisions the guard directory. After authorization:

1. Repeat the v9 pre-checks: identity, manager and metamodule hook digests, no pending module operation, and no diagnostic path. Record the `/dev/uinput` owner, mode, device numbers and SELinux label from the root shell as observations.
2. Check that `/data/adb/earkey_control` is absent. Create it as `0:0` mode `0700` with no files, and verify that no `stay-inactive` marker exists.
3. Install, reboot and read the records as in the v9 procedure. Success is `probe=uinput_open_ok` with `service_probe_exit=0`. If the open is denied, read only bounded AVC lines that name `uinput` and retain them privately. Do not propose any policy change without separate review.
4. Uninstall the diagnostic and reboot for cleanup. Remove `/data/adb/earkey_control` only if it is still an empty directory, and verify removal together with the usual diagnostic cleanup checks.

The run tests only open permission for the services hook on this kernel. It does not test uinput device creation, evdev, grabs, the EarKey Control module, passthrough or suppression. The on-phone escape-marker refusal is not exercised; host tests cover it.

## Service-context uinput open result — 10 November 2026

The maintainer authorized the v10 run, including provisioning and removing the guard directory. Pre-checks matched the v9 profile: the manager and metamodule hook digests were unchanged, no module operation was pending, and neither the diagnostic nor `/data/adb/earkey_control` existed. The root shell observed `/dev/uinput` as a character device 10:223, owner `uhid:uhid` (3011:3011), mode `0660`, label `u:object_r:uhid_device:s0`. `/data/adb/earkey_control` was created as `0:0` mode `0700`, labelled `u:object_r:adb_data_file:s0`, and left empty. The on-device ZIP digest matched `3e59f8da9582b40ce6c43b4d0fa83f8aa20cb7f8c8c4f3bac31d85bb5fb07997`. `0.0.10-preflight` was staged, its probe matched `bb9469338156678c53eec406903063bfe6c13a46e4377b5c6bfbdd72e24b2f42`, and the upload was removed.

The service-context uinput open check **Passed**. The services hook at 13.87 s verified the composite identity, with driver `full_version=v3.3.1-32726` recorded as an observation, and ran `probe-uinput`. It reported `launcher=resukisu_service`, `effective_uid=0`, `selinux_context=u:r:ksu:s0`, `selinux_enforcing=1`, `arch=aarch64`, `page_size=4096`, `probe=uinput_open_ok` and `service_probe_exit=0`. The boot-completed hook verified the identity and did not run the probe. A bounded search of `dmesg` and logcat found no AVC line naming `uinput`. The guard directory remained empty.

The diagnostic was marked for removal, and cleanup was verified after the reboot. The empty guard directory was removed, and the diagnostic paths and upload were gone. The original six modules, the metamodule link, the manager digest and Enforcing mode remained. Raw records remain private.

This establishes only that the `u:r:ksu:s0` services hook may open `/dev/uinput` for writing on this kernel. It does not test `UI_SET_*`/`UI_DEV_CREATE` ioctls, Android's recognition of a virtual device, evdev access, grabs, the EarKey Control module, passthrough or suppression. No additional SELinux policy was needed for this open.

## Service-context supervision drill plan — 10 November 2026

The next check proves the no-grab pause and deadline paths before any grab (step 4 of the physical order). After the composite gate and digest check pass, the v11 fixture runs the pinned `earkey-drill drill` from the services hook with a 30 s kill deadline. The packaged binary and digest file are named `earkey-drill` and `drill.sha256`; the result keeps the `service_probe_exit` key. The drill requires valid private storage and an open gate from `Store::recovery` and the `stay-inactive` marker. It checks that `supervisor.lock` is exclusive, then runs four scenarios with workers that never open an input device:

| Scenario | Worker | Pass condition |
| --- | --- | --- |
| `pause` | normal | `[Stop(Paused)]`, and the worker is gone |
| `kill` | ignores `SIGTERM` | `[Stop(Paused), Kill(Paused)]` after the 1 s pause deadline, and the worker is gone |
| `stall` | stops reporting after its first checkpoint | `Stop(Fault(ProgressStalled))` after the 500 ms progress deadline |
| `parent_death` | ignores `SIGTERM` | the worker dies when its supervising thread exits |

It prints each duration, removes `supervisor.lock`, and never writes policy. The limits (start 2 s, progress 500 ms, pause 1 s, kill 1 s, restart 200 ms) are drill values, not production deadlines.

The fixture is `0.0.11-preflight` (version code 11). The host suite has 23 passing tests. `earkey-control-service-preflight-v11.zip` is 595,871 bytes with SHA-256 `4fcfd5dc17dbe6c770a56c7fafb99940556df70f17f9ca5ce3ce1be600909712`; two builds were byte-identical. It embeds the drill ELF `156a0391538fc0ceec18fc2f68144136215d885d9a2696feb11f5a98d9bbc475` ([build record](build.md#stage-4-android-supervision-drill)).

```sh
python3 tools/package-service-probe.py \
  target/stage4/6db695d/earkey-drill-arm64-api31 \
  target/stage4/earkey-control-service-preflight-v11.zip
```

After authorization, follow the v10 procedure: pre-checks, an empty `0:0` `0700` `/data/adb/earkey_control`, install, reboot and record read, uninstall, cleanup reboot, and removal of the directory only if it is empty. Success is `drill=pass` with `service_probe_exit=0`. Also confirm that no `earkey-drill` process remains after the record read and after cleanup.

This tests supervision timing, parent death and `/proc` visibility in the `u:r:ksu:s0` services hook on this kernel. It does not test the real worker, evdev, uinput device creation, grabs, passthrough or suppression.

## Service-context supervision drill result — 10 November 2026

The maintainer authorized the v11 run, including provisioning and removing the guard directory. Pre-checks matched the v9 profile: the manager and metamodule hook digests were unchanged, no module operation was pending, and neither the diagnostic, `/data/adb/earkey_control` nor an `earkey-drill` process existed. A first broad `ps | grep earkey` count of 3 was a false match on `android.hardware.drm-service.clearkey`; later checks matched `earkey-drill` exactly. The guard directory was created as `0:0` mode `0700` and left empty. The on-device ZIP digest matched `4fcfd5dc17dbe6c770a56c7fafb99940556df70f17f9ca5ce3ce1be600909712`. `0.0.11-preflight` was staged with `earkey-drill` mode `0500`, its digest matched `156a0391538fc0ceec18fc2f68144136215d885d9a2696feb11f5a98d9bbc475`, and the upload was removed.

The service-context supervision drill **Passed**. The services hook at 13.93 s verified the composite identity, with driver `full_version=v3.3.1-32726` recorded as an observation, and ran `earkey-drill drill`:

| Scenario | Actions | Duration | Worker gone |
| --- | --- | --- | --- |
| `pause` | `[Stop(Paused)]` | 105 ms | yes |
| `kill` | `[Stop(Paused), Kill(Paused)]` | 1,074 ms | yes |
| `stall` | `[Stop(Fault(ProgressStalled))]` | 560 ms | stopped |
| `parent_death` | thread exit | 10 ms | yes |

It also reported `gate=open`, `lock_exclusive=true` and `lock_removed=true`, then `drill=pass` and `service_probe_exit=0`. The boot-completed hook verified the identity and did not run the drill. About 100 s after boot, the guard directory was empty and no `earkey-drill` process remained. A bounded `dmesg` search found no AVC line naming `earkey` or `ksu`.

Pause took 105 ms on the phone, against 11–12 ms in the container; kill took 1,074 ms against about 1,015 ms, and stall 560 ms against about 505 ms. The phone run happened during early boot. These are single drill measurements with drill limits, not production deadlines or a timing distribution.

The diagnostic was marked for removal, and cleanup was verified after the reboot. The empty guard directory was removed, and the diagnostic paths and upload were gone. The original six modules, the metamodule link, the manager digest and Enforcing mode remained, and no `earkey-drill` process existed. Raw records remain private.

This establishes that, in the `u:r:ksu:s0` services hook on this kernel, the adapter's process-group `SIGTERM`/`SIGKILL`, `PR_SET_PDEATHSIG`, `waitid(WNOWAIT)`, `/proc` group scan, `flock` lock and store-backed gate worked for a worker that opened no input device. It does not test the real worker, evdev, uinput device creation, grabs, passthrough or suppression.

## Service-context worker check plan — 10 November 2026

The next check runs the worker's read-only acquisition steps against the real earphone node from the services hook. After the composite gate and the binary digest check, the v12 hook computes the SHA-256 of `/system/usr/keylayout/Generic.kl` with toybox `sha256sum` under a 5 s deadline. It requires 64 lowercase hex characters, then runs `./earkey-worker check <digest>` under a 10 s deadline. An unavailable or malformed digest is recorded as `layout_digest=unavailable`, and the worker is not run.

`check` stops at the first failure and reports one `key=value` line per step:

| Step | Pass value |
| --- | --- |
| `evidence` | `xiaomi_pixelos_reforge20261110`, selected from the six properties and `/proc/version` |
| `inventory_nodes` | the node count; any node that cannot be observed fails |
| `enrollment` | `observed`: exactly one node enrolls under the evidence, from its live observation |
| `layout` | `generic`: no `Vendor_0020_Product_0b21*` or name-based `.kl`/`.idc` exists in the ODM, vendor, system or data directories |
| `targets` | `1`, at the observed instance, for an in-memory Block all policy |
| `open` | `ok`: the node opened read-only and is the same instance |
| `revalidate` | `ok`: the descriptor-side ID, name and capabilities and a fresh inventory still select the target |
| `neutral` | `yes`: `EVIOCGKEY` reports no pressed key |

It prints no serial, device name or key code, reads no event, takes no grab and writes nothing. The enrollment and policy exist only in memory. The fixture is `0.0.12-preflight` (version code 12). The host suite has 24 passing tests. `earkey-control-service-preflight-v12.zip` is 592,918 bytes with SHA-256 `1407369f28d35a03e9566c905743c052cde81c1e68714351548bc19eb6a95a75`; two builds were byte-identical. It embeds the worker ELF `7ee31586bf546e696c1e180d5766eaa541ab1628b3f35bd0d091a041c329a4c9` ([build record](build.md#stage-4-android-worker-check)).

After authorization, follow the v11 procedure with the earphones connected throughout, from before the reboot until the record read, and without pressing a button. Use wireless ADB, because the earphones occupy the USB-C port. Success is `check=pass` with `service_probe_exit=0`.

In v12 the digest is computed by the reviewed hook, not the worker. After v12 the maintainer chose the pinned `sha2` crate, and `earkey-worker check` now takes no argument and hashes `Generic.kl` itself. The v12 fixture still pins the earlier reviewed ELF, which takes the digest argument; the next fixture must use a rebuilt worker. This run does not test the uinput proxy, grabs, passthrough or suppression.

## Service-context worker check result — 10 November 2026

The maintainer authorized the v12 run. Pre-checks over wireless ADB matched the reforge profile: the manager and metamodule hook digests were unchanged, no module operation was pending, neither the diagnostic nor `/data/adb/earkey_control` existed, no `earkey-worker` process ran, exactly one earphone node was present, and the `Generic.kl` digest began `4d28d904`. The guard directory was created as `0:0` mode `0700` and left empty. The on-device ZIP digest matched `1407369f28d35a03e9566c905743c052cde81c1e68714351548bc19eb6a95a75`. `0.0.12-preflight` was staged with `earkey-worker` mode `0500`, its digest matched `7ee31586bf546e696c1e180d5766eaa541ab1628b3f35bd0d091a041c329a4c9`, and the upload was removed.

Wireless ADB did not return on its previous port after the reboot. The maintainer reconnected the phone by USB, which unplugged the earphones after boot; the services hook had already run by then. The earphones were present when the hook ran.

The service-context worker check **Passed**. The services hook at 13.22 s in `u:r:ksu:s0` verified the composite identity, with driver `full_version=v3.3.1-32726` recorded as an observation. It computed `layout_sha256=4d28d904e72ea3beb4d32d73437cf9e7f1ed297a436d51d53b1a248a5cd5f773` and ran `earkey-worker check`:

```text
evidence=xiaomi_pixelos_reforge20261110
inventory_nodes=6
enrollment=observed
layout=generic
targets=1
open=ok
revalidate=ok
neutral=yes
check=pass
```

The result was `service_probe_exit=0`. The boot-completed hook verified the identity and did not run the worker. At the record read the guard directory was empty, no `earkey-worker` process existed, and no AVC line naming `earkey`, `uhid` or `input_device` was found. Six input nodes existed at 13.22 s; more appeared later in boot, and the inventory enumerates nodes at the moment it runs.

The diagnostic was marked for removal, and cleanup was verified after the reboot. The empty guard directory was removed, and the diagnostic paths and upload were gone. The original six modules, the metamodule link, the manager digest and Enforcing mode remained. Raw records remain private.

This establishes that, from the `u:r:ksu:s0` services hook on the reforge kernel, the worker's inventory, environment, live enrollment, layout-override check, targeting, read-only open, `EVIOCG*` revalidation and `EVIOCGKEY` neutral check all succeeded against the real earphone node. It does not test the uinput proxy, grabs, passthrough, suppression, button mappings or preservation behavior.

## Service-context proxy check plan — 10 November 2026

The next check creates the uinput proxy described in [decision record 0004](decisions/0004-proxy-identity.md) from the services hook and observes Android's view of it, before any grab. After the composite gate and the binary digest check, the v13 hook runs `./earkey-worker proxy-check` under a 75 s deadline. The worker hashes `Generic.kl` itself, so the hook no longer computes a digest.

`proxy-check` first runs every `check` step. Only if they all pass does it:

1. Create the proxy from the descriptor-verified input ID and the evidence's three control keys.
2. Wait up to 45 s for its `/dev/input` node and for `system_server` to hold it open, reporting `proxy_ready_ms` and `proxy_event`.
3. Rescan the inventory and require the proxy to be virtual with no USB identity, the fixed name, the copied ID, event types `[0, 1, 4]`, the three keys and `MSC_SCAN`, and no possible enrollment (`proxy_excluded=yes`).
4. Run `/system/bin/dumpsys input` under a 10 s deadline with a 4 MiB limit, and print only the proxy's Event Hub classes, layout, key character map and configuration. It requires `KEYBOARD` and `EXTERNAL`, `/system/usr/keylayout/Generic.kl`, `ConfigurationFile: <none>` and the observed node path.
5. Destroy the proxy and require its node to disappear within 2 s (`proxy_teardown=ok`).

It ends with `proxy_check=pass`. A failed `check` creates no proxy. The proxy is destroyed on every exit path. Nothing grabs the physical node or writes an event to the proxy. The fixture is `0.0.13-preflight` (version code 13). The host suite has 23 passing tests. `earkey-control-service-preflight-v13.zip` is 692,644 bytes with SHA-256 `a524f1651fb97285abaeeeb92fa1cb13a04dc2156b56ab151f4344ac734c1b88`; two builds were byte-identical. It embeds the worker ELF `57178e5f0431f85ac0214505381a460b3cdb0a6953191f51bfea9aa085b4e895` ([build record](build.md#stage-4-android-worker-check)).

After authorization, follow the v12 procedure with the earphones connected from before the reboot until the record read, and no button pressed. Whether the `u:r:ksu:s0` domain may read `system_server`'s descriptors and run `dumpsys` from the services hook is unverified. A denial there is a recorded failure, not a reason to change policy.

## Service-context proxy check result — 10 November 2026

The maintainer authorized the v13 run and connected the phone over wireless ADB with the earphones plugged in. Pre-checks matched the reforge profile, with one earphone node and no proxy node present. The guard directory was created as `0:0` mode `0700` and left empty. The on-device ZIP digest matched `a524f1651fb97285abaeeeb92fa1cb13a04dc2156b56ab151f4344ac734c1b88`, and the staged worker matched `57178e5f0431f85ac0214505381a460b3cdb0a6953191f51bfea9aa085b4e895`. Wireless ADB returned on a new port after each reboot.

The service-context proxy check **Failed** at the exclusion comparison. The services hook at 13.32 s in `u:r:ksu:s0` passed every `check` step, from the reforge evidence to `neutral=yes`. It then reported `proxy=created`, `proxy_ready_ms=7163`, `proxy_event=event6` and `proxy_excluded=no`, then `proxy_check=fail` and `service_probe_exit=1`. The proxy was destroyed on that exit path; no proxy node or `earkey-worker` process remained at the record read, and the guard directory was empty. Filtered AVC lines named only `system_server` reads of unrelated properties and binder logs.

The cause is a comparison defect in the worker, not proxy behavior. The evidence lists the controls Center first (`0xa4`, `0x73`, `0x72`), while the inventory reports bitmap bits in ascending order. The v13 code compared the two orders directly. The fix (`ba9cd7d`) sorts the proxy keys once for creation and comparison, and adds a regression test.

The diagnostic was marked for removal, and cleanup was verified after the reboot. The guard directory was removed, and the diagnostic paths and upload were gone. The original six modules, the metamodule link, the manager digest and Enforcing mode remained, with no proxy node or worker process. Raw records remain private.

This run does establish that the services hook can create the decision-0004 proxy and that `system_server` opened it within 7,163 ms, which also shows that `u:r:ksu:s0` could read `system_server`'s descriptors. The proxy's inventory exclusion as reported, its Android classification and the explicit teardown check were not reached.

## Service-context proxy check rerun plan (v14) — 10 November 2026

v14 repeats the v13 procedure with the fixed worker. The ELF was built twice from `ba9cd7d` with identical results: 675,016 bytes, SHA-256 `7e04111588033f095b4257d367c4925b3d868af66ac23e30fdee769ea72897e5`. It passes the same pins and ELF checks: API 31, 16 KiB-aligned segments, only `libc` and `libdl`, all 82 imports in API 31, and no host paths. The fixture is `0.0.14-preflight` (version code 14). `earkey-control-service-preflight-v14.zip` is 694,868 bytes with SHA-256 `5041d8eebe7f5b872609bdc65d8aaf1e53b28303f6f98ae519d5297f0671ba65`; two builds were byte-identical. It needs a new maintainer authorization.

## Service-context proxy check rerun result (v14) — 10 November 2026

The maintainer authorized the v14 run. Pre-checks over wireless ADB matched the reforge profile, with one earphone node and no proxy node present. The guard directory was created as `0:0` mode `0700` and left empty. The on-device ZIP digest matched `5041d8eebe7f5b872609bdc65d8aaf1e53b28303f6f98ae519d5297f0671ba65`, and the staged worker matched `7e04111588033f095b4257d367c4925b3d868af66ac23e30fdee769ea72897e5`.

The service-context proxy check **Passed**. The services hook at 13.95 s in `u:r:ksu:s0` passed every `check` step (reforge evidence, 9 inventory nodes, live enrollment, one target, read-only open, revalidation, `neutral=yes`) and then reported:

```text
proxy=created
proxy_ready_ms=6865
proxy_event=event9
proxy_excluded=yes
proxy_classes=KEYBOARD | EXTERNAL
proxy_layout=/system/usr/keylayout/Generic.kl
proxy_kcm=/system/usr/keychars/Generic.kcm
proxy_configuration=<none>
proxy_android=ok
proxy_teardown=ok
proxy_check=pass
```

The result was `service_probe_exit=0`. The boot-completed hook verified the identity and did not run the worker. At the record read the guard directory was empty, no proxy node or `earkey-worker` process existed, and no AVC line naming `earkey`, `uhid`, `uinput`, `input_device`, `dumpsys` or `ksu` was found.

The diagnostic was marked for removal, and cleanup was verified after the reboot. The guard directory was removed, and the diagnostic paths and upload were gone. The original six modules, the metamodule link, the manager digest and Enforcing mode remained, with no proxy node or worker process. Raw records remain private.

This establishes, for the reference phone on the reforge kernel from the services hook, that the decision-0004 proxy can be created, that Android opens it within about 7 s of creation at 14 s uptime, that Android classifies it `KEYBOARD | EXTERNAL` with `Generic.kl` exactly like the physical earphones, that the worker inventory excludes it, and that teardown removes it. It does not test forwarded events, key mapping or action equivalence, repeat behavior, grabs, passthrough or suppression. Those are the Stage 4 passthrough gate. A single readiness sample at early boot is not a production deadline.

## Supervised passthrough session plan (v15) — 10 November 2026

This is the first Stage 4 passthrough session: grab the verified earphone collection and forward every event through the proxy, with no suppression, under the supervisor, for a bounded time.

- **Services hook:** after the composite gate and digest check, it runs the read-only `earkey-worker check` (30 s deadline).
- **Boot-completed hook:** after the same gate, it runs `earkey-worker supervise 120` (200 s kill deadline). ReSukiSU launches both stages without waiting for them. The supervisor starts `earkey-worker passthrough`, which:
  1. re-checks the gate;
  2. runs the read-only acquisition;
  3. creates the proxy and waits up to 15 s for Android readiness;
  4. grabs only if no key is down or queued;
  5. forwards until the supervisor pauses it at 120 s.
- **Success:** `supervise=pass`, which requires the actions `[Start, Stop(Paused)]`, no failure and a final `Inactive(Paused)` phase, plus `worker_end=Stopped` and `guard_exit=0`.

The fixture is `0.0.15-preflight` (version code 15). The host suite has 23 passing tests. `earkey-control-service-preflight-v15.zip` is 761,674 bytes with SHA-256 `3ab1612ceda3004375774a8b23a7ab0bdec7b6de1ef4db49f43210a91c8114c8`; two builds were byte-identical. It embeds the worker `e6a32fca5671f4d0831bb9615e5d7c7cf4adf2a0824a012fcf8767a4e93ed091` ([build record](build.md#stage-4-android-worker-check)).

### Maintainer protocol during the two-minute window

With the earphones connected before the reboot, unlock the phone after boot and start music playback. Then, within the two minutes after boot completes:
1. Press Center once and confirm playback pauses exactly once. Press it again and confirm it resumes.
2. Press Volume Up once and Volume Down once, and confirm exactly one volume step each.
3. Confirm audio kept playing through the earphones and the phone's own volume and power keys still work.
4. Report anything that acted twice, did not act, or behaved differently from normal.

Holds, double presses, calls and the microphone are later cases.

### Stopping early and recovery

- **Stopping early:** from a root shell, create the marker with `(umask 077; : > /data/adb/earkey_control/stay-inactive)`. The supervisor re-reads the gate every 50 ms and pauses the worker. Even a malformed marker closes the gate. Unplugging the earphones ends the worker with a disconnect fault and releases everything.
- **Recovery:** a worker crash releases the grab when its descriptors close, and the supervisor's kill path and the parent-death signal bound a hung worker. The session cannot outlive 120 s plus the pause and kill deadlines. The diagnostic is one-shot per installation, and cleanup follows the v14 procedure. Remove the marker if one was created, before removing the empty guard directory.

This session tests passthrough of short presses in the boot-completed context only. It is not suppression, module installation or acceptance.

## Supervised passthrough session result (v15) — 10 November 2026

The maintainer authorized the v15 run, had the earphones connected before the reboot, and confirmed readiness before the reboot was issued. Pre-checks over wireless ADB matched the reforge profile, with one earphone node and no proxy present. The guard directory was created as `0:0` mode `0700` and left empty. The on-device ZIP digest matched `3ab1612ceda3004375774a8b23a7ab0bdec7b6de1ef4db49f43210a91c8114c8`, and the staged worker matched `e6a32fca5671f4d0831bb9615e5d7c7cf4adf2a0824a012fcf8767a4e93ed091`.

The services hook passed the read-only `check` (`check=pass`, exit 0). The boot-completed hook at 27.18 s in `u:r:ksu:s0` ran `supervise 120`, and the session **Passed**:

```text
worker_check=pass
worker_proxy_ready_ms=22
worker_progress=1249
worker_end=Stopped
supervisor_stop_ms=366
supervisor_running_after_ms=203
supervisor_actions=[Start, Stop(Paused)]
supervisor_phase=Inactive(Paused)
supervisor_failures=0
supervisor_last_fault=none
supervise=pass
supervise_lock_removed=true
guard_exit=0
```

The worker's acquisition selected the reforge evidence and passed every step, including the descriptor revalidation and the neutral check. Android had already finished booting, so it opened the proxy in 22 ms. The worker reported 1,249 loop checkpoints at the 100 ms tick over the two-minute session. At the deadline the supervisor paused it, and the orderly stop with key release and ungrab took 366 ms. At the record read no proxy node or `earkey-worker` process existed, the guard directory was empty, and no AVC line naming `earkey`, `uhid`, `uinput`, `input_device` or `ksu` was found.

During the session the maintainer reports that all buttons were normal and acted once each. With music playing, Center paused and resumed once per press, Volume Up and Volume Down changed one step each, audio kept playing through the earphones, and the phone's own keys worked. This is a qualitative maintainer report of short presses through the proxy. No event counts were captured, and holds, double presses, calls and microphone routing were not exercised.

The diagnostic was marked for removal, and cleanup was verified after the reboot. The guard directory was removed, and the diagnostic paths and upload were gone. The original six modules, the metamodule link, the manager digest and Enforcing mode remained, with no proxy node or worker process. Raw records remain private.

This is the first physical passthrough evidence. On the reference phone, from the boot-completed hook, the supervised worker grabbed the verified collection, forwarded short Center and volume presses through the decision-0004 proxy with the expected single actions, and released cleanly on pause. Stage 4 still needs fault paths on the phone (worker crash, hang, unplug, early stop by the `stay-inactive` marker), holds and repeats, double presses, calls and the microphone, and a session started from the services stage. This is not suppression, module installation or support qualification.

## Supervised fault drill plan (v16) — 10 November 2026

v16 tests the Stage 4 failure paths on the phone during one supervised passthrough session. The boot-completed hook runs `earkey-worker supervise 600` under an 800 s kill deadline. The supervisor now reports `supervisor_started_uptime_ms` and a `supervisor_timeline` of every action and phase change with millisecond offsets, so externally injected faults can be matched to its behavior. The production deadlines are unchanged from v15. The diagnostic never persists the latch.

The fixture is `0.0.16-preflight` (version code 16). `earkey-control-service-preflight-v16.zip` is 783,898 bytes with SHA-256 `02cf742d9067e918e2d99176f95201250ecc681ae092adb00334a2c7bd3a729e`; two builds were byte-identical. It embeds the worker `9cd7ff90e5b9cdab1f7600c2a82226f395f5e66aec027b312eea83140ebc173e` ([build record](build.md#stage-4-android-worker-check)).

After authorization and the usual pre-checks, install and reboot with the earphones connected. Reconnect over wireless ADB. Record `/proc/uptime` before each injection from a root shell, and find the worker by exact command line, `earkey-worker passthrough`:

| Step | Injection | Expected supervisor timeline | Release checks within 5 s |
| --- | --- | --- | --- |
| F1 early stop | `(umask 077; : > /data/adb/earkey_control/stay-inactive)`; after 5 s, `rm` it | `Stop(BootDisabled)`, `Inactive(BootDisabled)`, then `Start` and `Running` after removal; no failure counted | worker process gone, proxy node gone |
| F2 crash | `kill -9 <worker>` | `Fault(UnexpectedExit)`, `WaitingToRestart`, then `Start` after the 5 s restart delay; failures 1 | worker gone, proxy node gone (uinput destroys the device when its descriptor closes) |
| F3 hang | `kill -STOP <worker>` | `Stop(Fault(ProgressStalled))` after about 1 s, `Kill(...)` after the 1 s pause deadline, exit confirmed, `WaitingToRestart`, `Start`; failures 2 | stopped worker gone, proxy node gone |
| F4 unplug while held | the maintainer holds Volume Up and unplugs the earphones while holding | a worker read or disconnect fault, `Fault(UnexpectedExit)`, failures 3, `PersistLatch`, which the diagnostic refuses (`supervisor_latch=error:Other`) | proxy node gone, no worker, and the maintainer confirms the volume did not keep changing and the phone keys work |

Between steps, the maintainer may press one earphone button to confirm passthrough resumed after each restart. A deviation from the expected timeline, a remaining proxy node or worker, a stuck key, or any boot or input problem is a failure to record, not a reason to repeat. Cleanup follows the v15 procedure. Remove the marker if one was left, then verify the guard directory is empty before removing it.

This tests supervision and release on the reference phone in the boot-completed context only. It does not test suppression, holds and repeat ownership, double presses, calls or the microphone.

## Supervised fault drill result (v16) — 10 November 2026

The maintainer authorized the v16 run and connected the earphones before installation. Pre-checks matched the reforge profile. The guard directory was created as `0:0` mode `0700`. The on-device ZIP digest matched `02cf742d9067e918e2d99176f95201250ecc681ae092adb00334a2c7bd3a729e`, and the staged worker matched `9cd7ff90e5b9cdab1f7600c2a82226f395f5e66aec027b312eea83140ebc173e`. The services hook passed the read-only `check`. The boot-completed hook at 27.36 s started `supervise 600`, with the supervisor starting at 27.73 s uptime. Wireless ADB reconnected at 92.6 s with a worker and proxy running. F1–F3 were injected from a root shell script piped over ADB, so nothing was written to the phone apart from the marker. F4 was performed by the maintainer.

| Step | Injection (uptime) | Supervisor timeline (uptime) | External release check |
| --- | --- | --- | --- |
| F1 early stop | marker created 113.97 s, removed 119.98 s | `Stop(BootDisabled)` 114.03 s, `Inactive(BootDisabled)` 114.56 s, `Start` 120.05 s, `Running` 120.25 s; no failure counted | worker and proxy gone by 114.94 s (0.97 s); restarted by 121.51 s |
| F2 crash | `kill -9` 124.58 s | `Fault(UnexpectedExit)` 124.91 s, `WaitingToRestart`, `Start` 129.95 s (5.04 s restart delay), `Running` 130.09 s; failures 1 | gone by 124.80 s (0.22 s); restarted by 131.09 s |
| F3 hang | `kill -STOP` 134.21 s | `Stop(Fault(ProgressStalled))` 135.20 s, `Kill` 136.21 s, exit confirmed 136.52 s, `Start` 141.55 s, `Running` 141.69 s; failures 2 | gone by 136.45 s (2.24 s); restarted by 142.83 s |
| F4 unplug while holding Volume Up | maintainer | the worker's read failed on the removed node (third failure); `PersistLatch` was refused by the diagnostic (`supervisor_latch=error:Other`), and `supervise` ended | the maintainer reports the volume stopped changing once unplugged and the phone keys work; afterwards no worker or proxy, no earphone node |

The proxy readiness after each restart was 15, 44, 12 and 22 ms, with Android up. At the record read no worker or proxy existed, the guard directory was empty, the lock had been removed, and no AVC line naming `earkey`, `uhid`, `uinput`, `input_device` or `ksu` was found.

Two reporting gaps were found, neither affecting release:
- After the unplug, `EVIOCGRAB` release on the removed node failed, and that error replaced the session fault in the worker report (`worker_end=error:Uncategorized`). The proxy keys had already been released.
- The latch path omitted the final status and step from the supervisor report.

Both are fixed with a test that removes the source device during a held key. The kernel releases the removed device's held keys (marking the report with value 1) before the read fails. The engine forwards that release, so the proxy key is released by the genuine release as well as by the session's cleanup.

The diagnostic was marked for removal before the cleanup reboot. Wireless debugging did not reappear until the maintainer re-enabled it later. Cleanup was then verified: the guard directory was removed, and the diagnostic paths and upload were gone. The original six modules, the metamodule link, the manager digest and Enforcing mode remained, with no proxy or worker. Raw records remain private.

This establishes, on the reference phone in the boot-completed context, that each injected fault produced the expected supervisor transition. Early stop through the marker released within 1 s and restarted on removal. A crash released within 0.22 s. A hang was detected by the progress deadline and released through the kill path within 2.3 s. Restarts were spaced by the 5 s deadline, the third failure requested the latch, and the unplug while held left no stuck key. It does not test a hang during a held key, suppression, double presses, calls or the microphone.

## Supervised gesture session plan (v17) — 10 November 2026

v17 checks gestures beyond single short presses with exact counts. On orderly stop, the worker now reports `worker_read_keys`, `worker_forwarded_keys` (per key, `down`/`repeat`/`up` record counts) and `worker_cleanup_releases`. The Stage 1 fixtures show that the earphones never emit repeat records (value 2): a hold is one press and one release, so Android's own key repeat drives held volume steps. The proxy, which has no `EV_REP`, should therefore behave identically, and passthrough should forward every record unchanged.

The hook is the same ten-minute boot-completed `supervise 600` as v16. The fixture is `0.0.17-preflight` (version code 17). `earkey-control-service-preflight-v17.zip` is 794,162 bytes with SHA-256 `669a0a98e53e0cd6514dce58fbe92b6d9ff3e108bff9a0a5881626fdc402517a`; two builds were byte-identical. It embeds the worker `5953973300dacd3be36f72fb4511ce13e8ebb70a53e868e760ef3d3107fc2cff` ([build record](build.md#stage-4-android-worker-check)).

After the session starts and music is playing, the maintainer performs, once each and with pauses between:

| Step | Gesture | Expected effect |
| --- | --- | --- |
| G1 | Center short press, twice | pause, then resume |
| G2 | Volume Up held about 2 s, then Volume Down held about 2 s | volume rises, then falls, continuously as normal |
| G3 | Center double press | next track |
| G4 | Center held about 2 s | assistant opens; dismiss it |
| G5 | Volume Up double press | two volume steps |

Then I create the `stay-inactive` marker to stop the worker in an orderly way, which makes it print its tally, and remove the marker. The expected tally is `0xa4` (Center) down 5, up 5; `0x73` (Volume Up) down 3, up 3; `0x72` (Volume Down) down 1, up 1; repeat 0 for every key; forwarded equal to read; and 0 cleanup releases. Extra counts would mean extra physical records, not a forwarding error, as long as forwarded equals read. The maintainer reports whether each effect matched normal behavior.

A hang during a held key is not part of v17. Volume saturates within about a second, so the effect would not be observable. The kernel's release of a removed device's held keys is covered by the container test.

## Supervised gesture session result (v17) — 11 November 2026

The maintainer authorized the v17 run. Pre-checks over wireless ADB matched the reforge profile, with one earphone node and no proxy present. The on-device ZIP digest matched `669a0a98e53e0cd6514dce58fbe92b6d9ff3e108bff9a0a5881626fdc402517a`, and the staged worker matched `5953973300dacd3be36f72fb4511ce13e8ebb70a53e868e760ef3d3107fc2cff`. The boot-completed session started, and wireless ADB reconnected at 104.8 s uptime with a worker and proxy running.

- **First worker (pid 3745):** the `stay-inactive` marker stopped it at 150.02 s (released in 0.59 s). It reported `worker_progress=1193`, `worker_read_keys=none`, `worker_forwarded_keys=none` and `worker_end=Stopped`. The maintainer later clarified that no button was pressed during this window, so an empty tally is the correct result. It restarted 0.19 s after the marker was removed.
- **Capture:** a 15 s read-only `getevent` capture, authorized by the maintainer, ran on only the earphone-related nodes while the second worker (pid 15673) held the grab. The earphone node (`event5` on that boot) showed no events, as expected while grabbed. The proxy (`event10`) showed every press as complete `MSC_SCAN`, key-down and key-up records.
- **Second worker:** the maintainer pressed buttons without following the exact G1–G5 protocol and reports everything behaved normally. On marker stop at 322.73 s (released in 0.58 s), it reported `worker_read_keys` equal to `worker_forwarded_keys`: `0x72` (Volume Down) 4/4, `0x73` (Volume Up) 7/7, `0xa4` (Center) 2/2, repeat 0, and cleanup releases 0.

This is the first counted evidence of passthrough on the phone. Every key record the worker read from the grabbed earphone node was forwarded to the proxy, the grabbed node was hidden from another reader, holds produced no repeat records (consistent with Stage 1, so Android's own key repeat drives held volume), and the maintainer reports normal effects. The specific G1–G5 gesture set was not performed as written, so individual double-press and hold outcomes are maintainer-reported rather than matched to a per-gesture count. v15's earlier short-press result remains a qualitative maintainer report without counts.

The session was ended by the cleanup reboot rather than its deadline, so no final supervisor report was written; partial records were read from `result.tmp`. Cleanup removed the marker, uninstalled the diagnostic, and rebooted. It then removed the supervisor's leftover `supervisor.lock`, which the reboot had prevented the supervisor from removing, and the empty guard directory. The original six modules, the metamodule link, the manager digest and Enforcing mode remained, with no proxy or worker. Raw records remain private.

## Stage 5 suppression session plan — 11 November 2026

Stage 5 checks Block all, then each separable group alone. The engine already implements per-press suppression, and the worker now takes a selection. To test several selections without a module install and reboot for each, these sessions run `earkey-worker supervise <seconds> <selection>` from the existing root ADB shell. That shell is the same `u:r:ksu:s0` domain the hooks recorded, with the same supervisor, worker, gate, lock, proxy and counting code. The services-hook launcher context was already established by v9–v17. The binary is the pinned ELF built twice from `f660d90`: 781,600 bytes, SHA-256 `ad1c3be5b51f4ff957222077341802c02a29d52e6863953e51102206243c5dba`. It passes the same NDK pins and ELF checks (API 31, 16 KiB-aligned segments, only `libc`/`libdl`, all 90 imports in API 31, no host paths), and is kept at `target/stage5/f660d90/earkey-worker-arm64-api31`.

**Procedure, after authorization and with the earphones connected:**
1. Repeat the reforge pre-checks.
2. Create the empty `0:0` `0700` guard directory.
3. Copy the ELF to a private root-owned temporary directory with mode `0500`, and check its digest on the phone.
4. For each selection below, run `supervise 60 <selection>` in the foreground. The session pauses itself at 60 s, prints the worker's tally and blocked effects, and removes its lock. Closing ADB kills it through the parent-death chain.
5. Afterwards, remove the temporary binary and directory, then the empty guard directory, and verify that no worker or proxy remains.

During each 60 s window, with music playing, the maintainer presses each earphone button once (Center, Volume Up, Volume Down) and one phone volume key, then reports what acted. The pass criteria are: blocked keys appear in `worker_read_keys` but not in `worker_forwarded_keys`, unblocked keys appear in both with equal counts, no repeat records, 0 cleanup releases, `supervise=pass`, and the maintainer reports that only the unblocked earphone buttons acted while the phone keys always worked.

| Run | Selection | Expected maintainer observation |
| --- | --- | --- |
| S1 | `block-all` | no earphone button acts; phone keys work |
| S2 | `center` | Center does nothing (no pause or assistant); volume buttons work |
| S3 | `volume-up` | only earphone Volume Up does nothing |
| S4 | `volume-down` | only earphone Volume Down does nothing |

Call-context effects of blocking Center (answer, end, reject, mute) are disclosed in the report but not exercised. Combinations and policy changes during a held key are later cases.

## Stage 5 suppression session results — 11 November 2026

The maintainer authorized S1–S4 from the root ADB shell (`u:r:ksu:s0`). Pre-checks matched the reforge profile, with the earphones connected and no proxy, worker, diagnostic or guard directory present. The guard directory was created as `0:0` mode `0700`. The pinned ELF was copied to a private `0700` temporary directory as a `0500 root:root` file, and its on-device digest matched `ad1c3be5b51f4ff957222077341802c02a29d52e6863953e51102206243c5dba`.

The first `block-all` run used a 60 s window. The maintainer pressed after the window had ended, so the worker read no key records while the buttons acted normally (nothing was blocked by then). That run is inconclusive and is not counted. Each later run used a 180 s window started only after the maintainer confirmed readiness.

| Run | Selection | Read (down/up, repeat 0 for all) | Forwarded | Supervisor | Maintainer |
| --- | --- | --- | --- | --- | --- |
| S1 | `block-all` | Volume Down 64/64, Volume Up 47/47, Center 37/37 | none | `[Start, Stop(Paused)]`, 0 failures, stop 421 ms, pass | no earphone button acted; phone volume key worked |
| S2 | `center` | Volume Down 2/2, Volume Up 1/1, Center 10/10 | Volume Down 2/2, Volume Up 1/1 | same, stop 366 ms, pass | Center did nothing; earphone volume and phone key worked |
| S3 | `volume-up` | Volume Down 1/1, Volume Up 105/105, Center 4/4 | Volume Down 1/1, Center 4/4 | same, stop 465 ms, pass | earphone Volume Up did nothing; Center, Volume Down and phone key worked |
| S4 | `volume-down` | Volume Down 86/86, Volume Up 2/2, Center 2/2 | Volume Up 2/2, Center 2/2 | same, stop 440 ms, pass | earphone Volume Down did nothing; Center, Volume Up and phone key worked |

Every run reported 0 cleanup releases and `worker_end=Stopped`, and removed its lock. The blocked-effect disclosure listed the evidence's effects for the blocked groups. For Center, that is play/pause, track change, assistant and wake, and call answer/end, mute and reject. The counts exceed one press per button; the maintainer pressed repeatedly in some windows. Every blocked press was read and dropped, and every unblocked press was forwarded exactly once.

Cleanup removed the temporary binary and directory and the empty guard directory. No proxy node remained. A first broad process count of 2 was a false match on the cleanup command's own arguments; the exact-name count was 0. The original six modules and Enforcing mode remained. Raw records remain private.

This is the first physical suppression evidence. On the reference phone, Block all and each separable group alone suppressed exactly the selected earphone controls at the event level, passed the others through unchanged, and left the phone's own keys working (partial evidence for T03, T04 and T10). The runs were from the root shell rather than a module hook. Call contexts for a blocked Center (T05, T08), combinations, a policy change during a held key (T15), and policy-driven selection from durable storage remain open.

## Descriptor comparison design

The pinned manager first searches its own inherited descriptors for the two specific KernelSU anonymous-inode names, preferring `ksu_driver_su`; if neither is present, it requests a descriptor through its special syscall and caches the result. A shell-side descriptor inventory cannot establish which descriptor the manager actually received: `exec` can close descriptors, and the manager can obtain one afterward. The pinned [acquisition code](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/ksucalls.rs#L82-L177) and [Linux descriptor information](https://man7.org/linux/man-pages/man5/proc_pid_fdinfo.5.html) were reviewed on 8 November 2026.

The fixture scans at most 16 descriptors in each hook shell, with a one-second deadline per target lookup. It records only counts for the two exact driver names, classified by close-on-exec state. An incomplete scan and missing or unreadable flags remain explicit; unrelated targets, arguments, environment, device events and personal logs are not recorded. The counts describe the hook shell, not the later manager process, and an anonymous-inode name, descriptor number, inode or mount identifier alone is not a driver identity. A direct reproduction of the special syscall or ioctl needs its own reviewed compatibility and safety analysis and separate device authorization. The composite identity gate remains in force for the metadata check.

## Manager-process observation assessment

Tracing a process changes its execution: `PTRACE_SYSCALL` stops it at syscall boundaries, and other stops can occur. Android's SELinux and seccomp policies can also prevent tracing; root alone does not establish permission. AOSP's boot-tracing example changes boot configuration and SELinux policy, which is outside this project's baseline. Do not make SELinux permissive, alter the kernel, trace an existing manager daemon, or infer untraced behavior from traced execution. Sources reviewed 8 November 2026: [Linux ptrace](https://man7.org/linux/man-pages/man2/ptrace.2.html), [AOSP strace guidance](https://source.android.com/docs/core/tests/debug/strace), and [Android SELinux](https://source.android.com/docs/security/features/selinux).

A future candidate must launch only its own verified, disposable `ksud debug version` child, apply a fixed deadline and terminate only that child and its tracer on timeout. It must allowlist the relevant acquisition and information-query operations and record bounded classifications rather than raw syscall arguments, pointers, unrelated paths or general process traces. Run an untraced comparison in the same stated context and report timing differences, permission denials, incomplete traces and unknowns. The tracer must not start global manager lifecycle commands or invoke the driver-acquisition syscall independently. Review its process, descriptor, signal and recovery behavior with synthetic host tests before building an Android artifact. Existing enforcing policy and the composite identity gate remain unchanged; device execution needs separate authorization.

`tools/trace_observer_model.rs` is a host-testable arm64 event-reduction model, not a tracer or an Android artifact. It classifies only the pinned fork's acquisition and information-query numbers, bounds its counters, and marks mismatched, unpaired and interrupted sequences incomplete. It does not inspect a real process or validate syscall-stop ordering. On the 4.14 reference kernel, a future backend cannot assume `PTRACE_GET_SYSCALL_INFO`, introduced in Linux 5.3; it must handle architecture-specific registers, seccomp and signal stops without mispairing them. `PTRACE_O_EXITKILL` applies after it is configured, so it does not replace independent cleanup during setup. [Pinned UAPI](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/uapi/supercall.h), [Linux ptrace](https://man7.org/linux/man-pages/man2/ptrace.2.html).

The model can be tested with `rustc --edition 2024 --test tools/trace_observer_model.rs -o /tmp/earkey-trace-model && /tmp/earkey-trace-model`. An isolated arm64 Linux check used the official Rust 1.99.1 Alpine image at digest `sha256:0a555e8b51945db91fd7b8411b4272bb5b8bfa04261d31eaeb2ada564bc02632`, resolved 8 November 2026; these synthetic tests do not exercise ptrace.

The trace runner crate (`earkey-trace-runner`, named `earkey-trace-supervisor` until 10 November 2026) also has a Linux/AArch64-only ptrace smoke test. On 8 November 2026 it passed as part of six crate tests in the official Rust 1.99.1 Alpine 3.24.4 arm64 image above, on the Docker LinuxKit kernel `7.0.14-linuxkit`; the container had no network, dropped capabilities, a read-only repository mount and a read-only Cargo cache. Command: `cargo test --offline --locked -p earkey-trace-supervisor -- --test-threads=1`. The fixture traces only its own disposable child, reads the AArch64 `NT_PRSTATUS` register set, and pairs the child’s `ioctl` entry and exit for the pinned `GET_INFO` request against fd `-1`, which must fail with `EBADF`. It never invokes the driver-acquisition syscall, opens a device, launches `ksud`, or records unrelated syscall data. The register layout follows the [Linux v4.14 arm64 ptrace UAPI](https://github.com/torvalds/linux/blob/v4.14/arch/arm64/include/uapi/asm/ptrace.h); ptrace stop and `PTRACE_O_EXITKILL` behavior follows [ptrace(2)](https://www.man7.org/linux/man-pages/man2/ptrace.2.html), reviewed 8 November 2026.

This is evidence only that the synthetic AArch64 ptrace sequence worked under that container kernel. It does not validate the phone’s custom 4.14.357 kernel, Android seccomp or SELinux policy, the manager’s traced behavior, syscall stop handling under signals or threads, or a production tracer. `PTRACE_O_EXITKILL` is set only after the child’s initial stop; the fixture’s parent-side cleanup guard is not an independent watchdog. Android execution and any manager observation remain unrun and separately authorized.

`earkey-trace-runner` is an experimental process-group cleanup component for the trace observer, separate from the input-worker `earkey-supervisor`. It is not a complete tracer or independent watchdog. It reserves the child PID using `waitid(WNOWAIT)` before signaling the owned group, bounds its polling and reports cleanup failure. Its unsafe caller contract requires exclusive child reaping; a future dedicated supervisor must enforce that contract. A successful return does not confirm all descendants have exited. Process spawn itself is outside its deadline, a failed signal or uninterruptible child can leave cleanup unconfirmed, and descendants that escape the group are not covered. A background reaper is best effort. Do not use this component on the phone until a separately supervised launcher, process ownership checks and failure tests close those gaps.
