# Stage 4 Android preflight

Stage 3 is merged. This increment provides a standalone capability probe for the measured arm64 Android target at API 31. It has no installer, boot hook, input reader, event injection, grab or virtual-device creation path. [Build and inspect the probe](build.md#stage-4-android-capability-probe)

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

The initial load check needs no button presses, reconnect cycles, installation or reboot. Device loading, service permissions and preservation tests are pending.
