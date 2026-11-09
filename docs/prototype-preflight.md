# Stage 4 Android preflight

Stage 3 is merged. The standalone capability probe targets arm64 Android at API 31. A separate temporary diagnostic fixture was built to run its metadata-only command from the intended launcher; its first authorized service-hook run stopped at the kernel-version guard before executing the probe (see the [service-context result](#service-context-diagnostic-result--8-november-2026)). [Build and inspect the probe](build.md#stage-4-android-capability-probe)

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

The diagnostic has its own module ID, `earkey_control_preflight`. Its ZIP contains the exact verified ELF, a digest file, a boot-mode installer check, `service.sh`, `skip_mount`, module metadata and the license. The manager's native ZIP installer handles staging and normal boot activation. The fixture guards for ReSukiSU userspace 35257, kernel version 35240, UAPI 6 and arm64 Android API 37. A matching installer or root-shell query does not establish that the service launcher reports the same kernel version.

The pinned fork's service launcher derives `KSU_KERNEL_VER_CODE` from its kernel information query, which caches the result within the process. A separate shell query is a different invocation. Do not weaken the service guard on the basis of the shell result; resolve any discrepancy and verify runtime provenance first. This is an engineering requirement informed by the [launcher](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/module/mod.rs) and [query implementation](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/ksucalls.rs), researched 8 November 2026. The pinned source alone does not prove the running kernel's implementation.

For a kernel-version guard failure only, the fixture can compare the supplied value with one fresh `ksud debug version` invocation in the service context. It first requires UID 0, the expected UAPI environment, a root-owned mode-0755 regular nonsymlink manager binary, and the SHA-256 of the arm64 executable in the [official packaged build](https://github.com/Baka-SU/BakaSU/actions/runs/41207077584) for the pinned revision. The packaged binary is distinct from the raw build artifact. Both the digest check and query have five-second deadlines, and only bounded ASCII-digit output is recorded. The guard still fails and the probe remains unexecuted. The fresh query is a separate process and does not prove why values differ or establish service permissions. The pinned [debug command](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/cli.rs#L821-L823) calls the version query; driver acquisition can include the fork's special syscall and a kernel information ioctl.

The pinned fork also invokes a module's `boot-completed.sh` from a later [boot-completed stage](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/init_event.rs#L193-L208). This fixture uses that separate, one-shot hook to repeat the bounded version query, including when its kernel-version guard passes. The hook never runs the probe. Each hook records a boot-uptime marker at context capture, its own shell's bounded SELinux label and mount and UTS namespace identifiers, or marks them unavailable. The stages launch without waiting, so compare the markers instead of assuming execution order. The hook records are separate processes; even matching recorded context fields do not establish complete context equivalence or prove why versions differ. The `guard_exit` field in the later result reports guard success, not query success.

The service reserves one attempt per installation in its own private `.preflight` directory and records `hook=entered` before checking the launcher guards. A rejected guard records its name in `guard_failure`. For a kernel-version failure, it also reports whether the supplied value is unset, empty, invalid or numeric; only ASCII digits up to ten characters are recorded. Other environment values are not recorded. After the guards pass, the hook checks the ELF SHA-256 and calls only `self-check`. The hash check and probe each have a five-second kill deadline. The hook writes the artifact digest, probe output and exit code to `.preflight/result` through a private temporary file. An existing attempt, disable or remove marker inhibits execution. A nonzero probe exit is evidence to retain. Inspect `.preflight/result.tmp` if the final result is absent. If both are absent after a bounded wait, the launcher may have skipped the hook or the hook may have failed before creating its private directory.

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

The service-context launcher check is **Not passed**. The services hook did not execute the probe, and the kernel-version values differ by stage: `32704` at 13.60 s and `35240` at 27.78 s. The cause is unresolved. Do not weaken the guard or substitute the later boot-completed pass for the services hook. A follow-up must establish why the services-stage query returns a different value and whether the intended service context can satisfy the guard before the probe runs. The hook context records `selinux_context` but not enforcing state; the separate shell-context observation reported Enforcing, but that is not a service-hook record. The complete zero-count descriptor scan describes only the hook shell, not the manager process. No service-context probe, evdev access, uinput operation, grab, passthrough, suppression, audio, microphone or physical behavior is established.

The bounded raw records are retained privately. The temporary upload was removed after installation. `/data/adb/ksud module uninstall earkey_control_preflight` created the `remove` marker; after the authorized cleanup reboot, the active and staged module paths, both private record directories, the temporary upload and all pending markers were absent. The module set returned to the original six and the `magic_mount_rs` metamodule symlink was unchanged. No phone serial or temporary ADB detail is recorded here.

## Descriptor comparison design

The pinned manager first searches its own inherited descriptors for the two specific KernelSU anonymous-inode names, preferring `ksu_driver_su`; if neither is present, it requests a descriptor through its special syscall and caches the result. A shell-side descriptor inventory cannot establish which descriptor the manager actually received: `exec` can close descriptors, and the manager can obtain one afterward. The pinned [acquisition code](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/ksucalls.rs#L82-L177) and [Linux descriptor information](https://man7.org/linux/man-pages/man5/proc_pid_fdinfo.5.html) were reviewed on 8 November 2026.

The fixture scans at most 16 descriptors in each hook shell, with a one-second deadline per target lookup. It records only counts for the two exact driver names, classified by close-on-exec state. An incomplete scan and missing or unreadable flags remain explicit; unrelated targets, arguments, environment, device events and personal logs are not recorded. The counts describe the hook shell, not the later manager process, and an anonymous-inode name, descriptor number, inode or mount identifier alone is not a driver identity. A direct reproduction of the special syscall or ioctl needs its own reviewed compatibility and safety analysis and separate device authorization. The existing kernel-version guard remains unchanged.

## Manager-process observation assessment

Tracing a process changes its execution: `PTRACE_SYSCALL` stops it at syscall boundaries, and other stops can occur. Android's SELinux and seccomp policies can also prevent tracing; root alone does not establish permission. AOSP's boot-tracing example changes boot configuration and SELinux policy, which is outside this project's baseline. Do not make SELinux permissive, alter the kernel, trace an existing manager daemon, or infer untraced behavior from traced execution. Sources reviewed 8 November 2026: [Linux ptrace](https://man7.org/linux/man-pages/man2/ptrace.2.html), [AOSP strace guidance](https://source.android.com/docs/core/tests/debug/strace), and [Android SELinux](https://source.android.com/docs/security/features/selinux).

A future candidate must launch only its own verified, disposable `ksud debug version` child, apply a fixed deadline and terminate only that child and its tracer on timeout. It must allowlist the relevant acquisition and information-query operations and record bounded classifications rather than raw syscall arguments, pointers, unrelated paths or general process traces. Run an untraced comparison in the same stated context and report timing differences, permission denials, incomplete traces and unknowns. The tracer must not start global manager lifecycle commands or invoke the driver-acquisition syscall independently. Review its process, descriptor, signal and recovery behavior with synthetic host tests before building an Android artifact. Existing enforcing policy and the version guard remain unchanged; device execution needs separate authorization.

`tools/trace_observer_model.rs` is a host-testable arm64 event-reduction model, not a tracer or an Android artifact. It classifies only the pinned fork's acquisition and information-query numbers, bounds its counters, and marks mismatched, unpaired and interrupted sequences incomplete. It does not inspect a real process or validate syscall-stop ordering. On the 4.14 reference kernel, a future backend cannot assume `PTRACE_GET_SYSCALL_INFO`, introduced in Linux 5.3; it must handle architecture-specific registers, seccomp and signal stops without mispairing them. `PTRACE_O_EXITKILL` applies after it is configured, so it does not replace independent cleanup during setup. [Pinned UAPI](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/uapi/supercall.h), [Linux ptrace](https://man7.org/linux/man-pages/man2/ptrace.2.html).

The model can be tested with `rustc --edition 2024 --test tools/trace_observer_model.rs -o /tmp/earkey-trace-model && /tmp/earkey-trace-model`. An isolated arm64 Linux check used the official Rust 1.99.1 Alpine image at digest `sha256:0a555e8b51945db91fd7b8411b4272bb5b8bfa04261d31eaeb2ada564bc02632`, resolved 8 November 2026; these synthetic tests do not exercise ptrace.

The supervisor crate also has a Linux/AArch64-only ptrace smoke test. On 8 November 2026 it passed as part of six crate tests in the official Rust 1.99.1 Alpine 3.24.4 arm64 image above, on the Docker LinuxKit kernel `7.0.14-linuxkit`; the container had no network, dropped capabilities, a read-only repository mount and a read-only Cargo cache. Command: `cargo test --offline --locked -p earkey-trace-supervisor -- --test-threads=1`. The fixture traces only its own disposable child, reads the AArch64 `NT_PRSTATUS` register set, and pairs the child’s `ioctl` entry and exit for the pinned `GET_INFO` request against fd `-1`, which must fail with `EBADF`. It never invokes the driver-acquisition syscall, opens a device, launches `ksud`, or records unrelated syscall data. The register layout follows the [Linux v4.14 arm64 ptrace UAPI](https://github.com/torvalds/linux/blob/v4.14/arch/arm64/include/uapi/asm/ptrace.h); ptrace stop and `PTRACE_O_EXITKILL` behavior follows [ptrace(2)](https://www.man7.org/linux/man-pages/man2/ptrace.2.html), reviewed 8 November 2026.

This is evidence only that the synthetic AArch64 ptrace sequence worked under that container kernel. It does not validate the phone’s custom 4.14.357 kernel, Android seccomp or SELinux policy, the manager’s traced behavior, syscall stop handling under signals or threads, or a production tracer. `PTRACE_O_EXITKILL` is set only after the child’s initial stop; the fixture’s parent-side cleanup guard is not an independent watchdog. Android execution and any manager observation remain unrun and separately authorized.

`earkey-trace-supervisor` is an experimental process-group cleanup component, not a complete tracer or independent watchdog. It reserves the child PID using `waitid(WNOWAIT)` before signaling the owned group, bounds its polling and reports cleanup failure. Its unsafe caller contract requires exclusive child reaping; a future dedicated supervisor must enforce that contract. A successful return does not confirm all descendants have exited. Process spawn itself is outside its deadline, a failed signal or uninterruptible child can leave cleanup unconfirmed, and descendants that escape the group are not covered. A background reaper is best effort. Do not use this component on the phone until a separately supervised launcher, process ownership checks and failure tests close those gaps.
