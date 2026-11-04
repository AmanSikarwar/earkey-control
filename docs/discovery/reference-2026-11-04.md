# Reference discovery checkpoint — 4 November 2026

**Stage 1 incomplete. The maintainer explicitly approved this sanitized checkpoint for public review on 4 November 2026.** Remaining acceptance gates are tracked in [issue #4](https://github.com/AmanSikarwar/earkey-control/issues/4). These are unfiltered physical baseline observations, not module acceptance results or an enrolled profile. No installable module or runtime artifact exists.

The maintainer authorized read-only access to the wireless ADB connection after merging [PR #3](https://github.com/AmanSikarwar/earkey-control/pull/3). Initial button collection used source context `363bf13e10125cf75b19df7ef1fdd28340c1daec`; the later root follow-up used `dc76ba0dd9b548ae538deb4447ce35c00fd44ab2`. No installation, exclusive grab, virtual-device creation, event injection, root-manager setting change, SELinux change or reboot was performed by the agent. The maintainer enabled ADB root before the follow-up. Recordings are bounded; this report distinguishes completed observations from remaining tests.

## Measured environment

| Field | Observed value / boundary |
| --- | --- |
| Model / device properties | M2101K6P; product device sweetin; vendor device sweet; product custom_sweet. No market-variant name inferred |
| ROM | `PixelOS_sweet-17.0-20261030-2116`; net.pixelos.version=seventeen; build type ci |
| Android / framework | Release 17, API 37; system/product/system_ext/vendor version properties agree, and the android framework package reports versionName 17 / versionCode 37 |
| Build / patch | CP2A.260605.016; incremental 1793394994; security patch 2026-10-05 |
| Fingerprint discrepancy | Reported build fingerprint/description retains a Redmi Android 13 / TKQ1.221013.002 / V14.0.9.0.TKFMIXM identity. Do not infer the running framework is Android 13 from this; record both observations without assuming the reason |
| Kernel | `4.14.357-openela-VantomKernel`; build `#1 SMP PREEMPT Sun Oct 11 07:45:56 UTC 2026` |
| ABI / page size | aarch64; arm64-v8a, armeabi-v7a and armeabi advertised; page size 4096 bytes. No Android binary was built or load-tested |
| SELinux / ADB context | Initially UID 2000 / `u:r:shell:s0`; after the maintainer enabled ADB root, measured UID 0 / `u:r:ksu:s0`. SELinux remained Enforcing |
| Installed root-manager app | `com.resukisu.resukisu`, versionName `v4.2.0-rc3`, versionCode 35257 |
| Root userspace | `/system/bin/su` now visible; su reports `4.2.0-rc3-86-g516d3e87:KernelSU`, code 35257. `/data/adb/ksud --version` reports `4.2.0-rc3-86-g516d3e87`, UAPI 6 |
| Root kernel | Installed ksud's read-only debug version/info queries report version 35240, UAPI 6, runtime_mode built-in, lkm=false and late_load=false. Its full_version field is empty; no kernel source revision is inferred |
| uinput/evdev prerequisites | /dev/uinput is character 10:223, uhid:uhid, mode 0660, label `u:object_r:uhid_device:s0`; sysfs confirms DEVNAME=uinput. Kernel configuration reports CONFIG_INPUT_EVDEV=y and CONFIG_INPUT_UINPUT=y. Root read/write access predicates succeed for uinput and the candidate event node; no uinput open/ioctl/write or actual module-service usability test was performed |

The first environment command ended with exit 127 because its final su queries failed; its preceding property/kernel observations succeeded. The maintainer later explained that root had been disabled for ADB and enabled it. A first follow-up found no wireless device and ran no phone command; after the maintainer reconnected wireless ADB, UID 0 and the root versions above were verified. This transport reconnection was not an earphone reconnect-cycle test. No adb root toggle or root-manager configuration change was performed by the agent.

The root follow-up could read module metadata and narrowly inspect lifecycle scripts as data. That limited static review found no obvious input-remapping reference; it is not proof that all dynamic conflicts are absent. Module/app identities remain in private evidence. No module script or lifecycle event was executed. A root shell in the ksu domain does not replace testing the actual future module hook and forwarding semantics.

## Candidate collection and exclusions

The current diagnostic candidate is named **Generic Xiaomi Type-C Earphones 1**. Measured evdev input identifiers are bus `0x0003`, vendor `0x0020`, product `0x0b21`, version `0x0201`. Its sysfs ancestry passes through USB, interface `1.3`, and a matching HID collection. Its runtime event number was 9 in this session; that number is not a saved identity or selection rule.

Root-readable USB descriptors confirm vendor/product `0x0020:0x0b21`, USB bcdDevice `0x0100`, and product Xiaomi Type-C Earphones 1. Audio interfaces 00/01/02 use snd-usb-audio, while interface 03 has class 03/HID and uses usbhid. The USB bcdDevice value is a separate namespace from the evdev input version `0x0201`. ALSA exposes the accessory as a separate USB card with playback and capture endpoints; their audio quality and call behavior are still untested. USB host-controller descriptors contain a different kernel-release string from uname; both are retained privately and no underlying source identity is inferred from that discrepancy.

Declared capabilities are EV_KEY with KEY_MUTE, KEY_VOLUMEDOWN, KEY_VOLUMEUP, KEY_MENU, KEY_NEXTSONG, KEY_PLAYPAUSE, KEY_PREVIOUSSONG and KEY_VOICECOMMAND, plus EV_MSC/MSC_SCAN. No touch, relative axis, absolute axis or switch capability appeared on this collection. These are advertised capabilities; only the three measured button signals below are observed physical mappings.

Android resolves `/system/usr/keylayout/Generic.kl` and Generic.kcm for the candidate. The layout read in this session has SHA-256 `4d28d904e72ea3beb4d32d73437cf9e7f1ed297a436d51d53b1a248a5cd5f773`. It was read, never modified. The key collection's Android HasMic flag is not evidence about the separate audio microphone path.

Observed non-target collections include phone power/volume nodes qpnp_pon and gpio-keys, the goodix touchscreen, haptic input, fingerprint/navigation virtual nodes uinput-fpc and uinput-fortsense, and the internal sound-card headset/button/connection-switch nodes. None was recorded as an event stream, grabbed or enrolled. A similar media-key capability on an internal sound node does not establish accessory ownership.

The candidate disappeared after the maintainer unplugged the earphones and returned with matching inspected input attributes after one controlled reconnect. The Android audio inventory separately showed USB-headset output and input endpoints. This supports the observed USB control/audio relationship, but does not prove listening quality, microphone recording or call routing. Only **one controlled reconnect** is measured; initial attached-state inventory is not counted as another insertion. The maintainer has explicitly waived further physical reconnect-cycle repetitions for this reference setup under [D09](../project-contract.md#reference-validation-amendment--4-november-2026). Untested repetitions are waived, not passed.

## Button → Linux/HID → Android → action observations

Linux codes and scan values below are hexadecimal. Android names come from the actual selected layout, not numeric Linux-code substitution. The maintainer confirmed the three physical controls: Center, Volume Up and Volume Down.

| Physical gesture/context | Linux key / MSC_SCAN | Selected Android mapping | Evidence and observed action |
| --- | --- | --- | --- |
| Volume Up short presses, screen unlocked, media setup | KEY_VOLUMEUP `0x0073`; scan `0x000c00e9` | VOLUME_UP | 5 presses + 5 releases, no repeat value; user confirmed phone volume increased |
| Volume Down short presses, screen unlocked | KEY_VOLUMEDOWN `0x0072`; scan `0x000c00ea` | VOLUME_DOWN | 5 presses + 5 releases, no repeat value; user confirmed phone volume decreased |
| Center short-press window, media setup | KEY_PLAYPAUSE `0x00a4`; scan `0x000c00cd` | MEDIA_PLAY_PAUSE | 7 presses + 7 releases; the maintainer subsequently confirmed seven intentional presses and working play/pause. The captured count matches; no discrepancy remains |
| Volume holds window, media setup | Repeated complete KEY_VOLUMEUP/KEY_VOLUMEDOWN pairs; no EV_KEY value 2 observed | VOLUME_UP / VOLUME_DOWN | User confirmed volume kept changing while held. The window contains 37 Up and 12 Down pairs; those are raw sequence counts, not physical hold counts. Incidental actions are excluded from attribution at the maintainer's request, so this is not a clean five-hold qualification |
| Complete sustained sets, double gestures, locked/off and calls | Not fully measured | Not established | Do not offer independent call/assistant/track semantics or claim these contexts passed |

An initial 45-second non-PTY capture produced zero event bytes despite a subsequent user report of pressing all three buttons five times, in Center → Volume Down → Volume Up order. Its expected timeout exit was 124; absence of data is an inconclusive capture, not proof of blocked or absent button signals. Both two-second verbosity probes returned device headers, with and without a PTY. Later PTY-backed recordings received events. The initial failure's cause is unresolved and remains in private evidence; it is not silently replaced by the later recordings.

The completed short-press captures retained all scan/key/synchronization records: 102 events and 34 complete SYN_REPORT frames, no outstanding pressed key at their ends, and no observed SYN_DROPPED. This validates those short sequences only; it does not test overrun recovery or a filter implementation. The later 90-second held-volume window ended at its timeout; it also contains incidental input and is retained privately rather than promoted to a complete per-gesture fixture. The private reader used preflight identity checks, a remote hard timeout, a host deadline, a 64 KiB limit and a visible Stop/Done interaction. Its parser self-checks and short-fixture state assertions passed. No capture is active at the checkpoint handoff.

## Evidence and artifact identities

- Runtime module artifact: **none**. Initial short-press observations used source context `363bf13e10125cf75b19df7ef1fdd28340c1daec`; root/held-volume follow-up used `dc76ba0dd9b548ae538deb4447ce35c00fd44ab2`. Existing phone tools performed collection; no EarKey binary ran on the phone.
- Sanitized, relative-time [physical event fixture](../../tests/fixtures/discovery/reference-2026-11-04-short-presses.json): SHA-256 `bb718d4d3c213281065d4f91a0e83a7d9507b892e057b997ab959776178175fe`. Only its Center observation text changed after the maintainer confirmed seven intentional presses; event arrays, original capture metadata and raw digests are unchanged. The earlier published fixture digest `22a4de40222f9ac26bba4962edabe260933010a4f3c563d801a126687fa38be2` remains valid for that historical revision. This is diagnostic test data, not a policy/profile or acceptance certificate.
- Volume Up raw event record SHA-256: `d6b99428b17c06201475c043214999f81e21822bf1964b342c8cd5fcaa85f638`; Volume Down/Center raw event record: `489a86ad43adb36de79d16d00ff6c0927caab8655c7f11557dec5d609fa316d0`.
- Initial short-press reader source SHA-256: `d789a894dea1b14dda82212d872138c42e5d6d26c161ab867127485c6f2cf22f`. This temporary diagnostic reader is not the production identity/filter implementation; the separate root follow-up manifest binds its own reader snapshot.
- Private checkpoint manifest SHA-256: `1b2cf8b506fe318bdab38dd6aaed8abce3b386a1c6938ec744767d8de270a86d`, covering 103 evidence files / 117,586 bytes before the manifest itself. Raw records are under ignored private storage with mode 0600 inside private directories. Transport identifiers, serial/unique IDs, unrestricted mapping/audio diagnostics and incidental logs are excluded from this report/fixture.
- Private root-metadata manifest snapshot SHA-256: `21fd59cd14942937342f055a1a414dd7dc262e468cb8dcf5ff9464fc2f339594`, covering 50 files / 22,180 bytes before the manifest and before subsequent gesture recordings. It records read-only root/tool/USB/input/audio/permission metadata, not successful input interception or uinput forwarding.
- Private root/held-volume follow-up manifest SHA-256: `e6d8d93e95f132acdf3bbfec9d36c4a7222d07f912c9a7614497159778bf08b7`, covering 56 files / 40,412 bytes before the manifest. The mixed held-volume stream remains private and outside the published short-press fixture; its raw digest is `d6a72bd555d8126bd0d15598dccc796806175e9b6f7a831e9afbc469a1c96b97`. No incidental action is attributed to a new fault or feature.

Source timestamps, exit/stop reasons and raw digests remain in the private records. Public fixture event timestamps are relative; collection UTC metadata is retained for traceability. The initial public issue attempt was rejected by automatic approval review because device-derived disclosure needed specific consent; nothing was posted then. After reviewing the concrete report, fixture and prepared issue, the maintainer replied "you can publish" on 4 November. [Issue #4](https://github.com/AmanSikarwar/earkey-control/issues/4) was then created under that authorization. The approved subset discloses model/build/accessory details and sanitized button sequences; raw diagnostics, serial/unique IDs and network identifiers remain private.

## Remaining Stage 1 gates and backend decision

**The mountless grab + uinput architecture remains a candidate, not an approved device backend.** Mechanical scoping and root/kernel prerequisites are measured. Boot/duplicate identity behavior, actual module-service/forwarding behavior and remaining preservation/context evidence are unproven. No profile is enrolled and no automatic activation is eligible.

- Further physical reconnect repetitions are waived by the maintainer; one measured cycle is the current identity-stability evidence. Keep software reconnect handling/revalidation and host tests; boot stability remains unmeasured and no reboot is authorized.
- Complete clean five-observation sustained/repeat/double sets and contexts beyond the limited held-volume window. The confirmed seven intentional Center presses resolve the short-press count question. Explain shared playback/call/assistant effects from actual measurements; do not diagnose incidental actions the maintainer asked to exclude from this test.
- Cover paused, locked/off and consenting/local incoming/ongoing calls; obtain baseline microphone recording, listening/call routing and plug/unplug behavior.
- Explicitly test phone power/volume/touch and an unrelated input device; capture a defined idle/battery baseline. Incidental normal screen use is not a completed preservation test.
- Root/kernel/context and limited module-conflict metadata are now measured. Actual module-service execution, uinput creation/readiness/mapping and exclusive interception remain later authorized prototype gates; successful root metadata/access predicates do not prove them.
- Make the explicit eligible/unsupported native-backend decision before dependent implementation. All module acceptance T01–T28, passthrough, suppression, crash/hang, recovery, installation and manager lifecycle tests remain Not run.

## Research provenance

Consulted on 4 November 2026: official [ADB manual/source](https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/user/adb.1.md) through Context7's AOSP ADB index, [AOSP getevent](https://source.android.com/docs/core/interaction/input/getevent), [AOSP key layouts](https://source.android.com/docs/core/interaction/input/key-layout-files), [AOSP SELinux](https://source.android.com/docs/security/features/selinux), and [KernelSU module guide](https://kernelsu.org/guide/module.html) through `/tiann/kernelsu`. Runtime command help was checked for the phone's getevent and Toybox 0.8.13-android timeout. Help exits 1/125 are retained as usage-query results.

Context7 did not index ReSukiSU. Its [official project introduction](https://resukisu.org/guide/introduce) was consulted; the old [upstream repository URL](https://github.com/ReSukiSU/ReSukiSU) currently redirects to Baka-SU/BakaSU. That moving upstream is not treated as the installed v4.2.0-rc3 API contract. The installed ksud version/help and documented read-only debug version/info commands supplied the root follow-up facts; no lifecycle, policy or feature-setting command was invoked. Exact matching source provenance and actual module-service/forwarding behavior remain unresolved. No root-manager upgrade or replacement is proposed.
