# Stage 4 Android preflight

Stage 3 is merged. The standalone capability probe targets arm64 Android at API 31. A separate temporary diagnostic fixture runs its metadata-only command from the intended launcher. [Build and inspect the probe](build.md#stage-4-android-capability-probe)

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

## Temporary service diagnostic

The diagnostic has its own module ID, `earkey_control_preflight`. Its ZIP contains the exact verified ELF, a digest file, a boot-mode installer check, `service.sh`, `skip_mount`, module metadata and the license. The manager's native ZIP installer handles staging and normal boot activation. The fixture guards for ReSukiSU userspace 35257, kernel version 35240, UAPI 6 and arm64 Android API 37. A matching installer or root-shell query does not establish that the service launcher reports the same kernel version.

The pinned fork's service launcher derives `KSU_KERNEL_VER_CODE` from its kernel information query, which caches the result within the process. A separate shell query is a different invocation. Do not weaken the service guard on the basis of the shell result; resolve any discrepancy and verify runtime provenance first. This is an engineering requirement informed by the [launcher](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/module/mod.rs) and [query implementation](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/ksucalls.rs), researched 8 November 2026. The pinned source alone does not prove the running kernel's implementation.

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
3. Reboot normally and wait for the existing ADB root route. Read only `/data/adb/modules/earkey_control_preflight/.preflight/result` for this check. A successful test records the reviewed digest, `launcher=resukisu_service`, `probe=self_check_ok`, `service_probe_exit=0`, effective UID, SELinux context, enforcing state and ABI. Compare the actual context with the shell observation. Retain the sanitized record privately.
4. Mark only `earkey_control_preflight` for removal with `/data/adb/ksud module uninstall earkey_control_preflight` and verify its `remove` marker. The exact fork skips removed modules and prunes them during the next normal boot. Reboot for cleanup and verify the active and staged paths, including the private result, are gone. [Fork boot and pruning sequence](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/init_event.rs)

Use the diagnostic result to establish the service launcher context. Device-node permissions, independent worker supervision and physical passthrough require their own checks.
