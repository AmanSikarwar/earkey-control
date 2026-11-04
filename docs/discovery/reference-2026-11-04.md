# Reference discovery checkpoint — 4 November 2026

**Stage 1 incomplete. Local review draft; publication of this device-derived material is awaiting explicit approval.** These are unfiltered physical baseline observations, not module acceptance results or an enrolled profile. No installable module or runtime artifact exists.

The maintainer authorized read-only access to the wireless ADB connection after merging [PR #3](https://github.com/AmanSikarwar/earkey-control/pull/3). Collection used the reviewed source at `363bf13e10125cf75b19df7ef1fdd28340c1daec`. No installation, exclusive grab, virtual-device creation, event injection, root-manager setting change, SELinux change or reboot was performed. Recording stopped when the requested windows ended; no capture is active at this checkpoint.

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
| SELinux / ADB context | Enforcing; UID 2000, `u:r:shell:s0`; input group available |
| Installed root-manager app | `com.resukisu.resukisu`, versionName `v4.2.0-rc3`, versionCode 35257. Installation metadata is not proof of the active root kernel version |
| Root access from ADB | `su` not visible on PATH or inspected standard paths; su version commands failed with exit 127; /data/adb and selected procfs files were denied. Root implementation/kernel version, installed modules and intended service context remain unverified |
| uinput node | /dev/uinput exists, character 10:223, owner/group uhid:uhid, mode 0660, label `u:object_r:uhid_device:s0`. No open/ioctl/write or service-domain usability test was performed |

The first environment command ended with exit 127 because its final su queries failed; its preceding property/kernel observations succeeded. A missing su command is not a conclusion that the phone is unrooted. No adb root request or root-manager configuration change was attempted.

## Candidate collection and exclusions

The current diagnostic candidate is named **Generic Xiaomi Type-C Earphones 1**. Measured evdev input identifiers are bus `0x0003`, vendor `0x0020`, product `0x0b21`, version `0x0201`. Its sysfs ancestry passes through USB, interface `1.3`, and a matching HID collection. Its runtime event number was 9 in this session; that number is not a saved identity or selection rule.

Declared capabilities are EV_KEY with KEY_MUTE, KEY_VOLUMEDOWN, KEY_VOLUMEUP, KEY_MENU, KEY_NEXTSONG, KEY_PLAYPAUSE, KEY_PREVIOUSSONG and KEY_VOICECOMMAND, plus EV_MSC/MSC_SCAN. No touch, relative axis, absolute axis or switch capability appeared on this collection. These are advertised capabilities; only the three measured button signals below are observed physical mappings.

Android resolves `/system/usr/keylayout/Generic.kl` and Generic.kcm for the candidate. The layout read in this session has SHA-256 `4d28d904e72ea3beb4d32d73437cf9e7f1ed297a436d51d53b1a248a5cd5f773`. It was read, never modified. The key collection's Android HasMic flag is not evidence about the separate audio microphone path.

Observed non-target collections include phone power/volume nodes qpnp_pon and gpio-keys, the goodix touchscreen, haptic input, fingerprint/navigation virtual nodes uinput-fpc and uinput-fortsense, and the internal sound-card headset/button/connection-switch nodes. None was recorded as an event stream, grabbed or enrolled. A similar media-key capability on an internal sound node does not establish accessory ownership.

The candidate disappeared after the maintainer unplugged the earphones and returned with matching inspected input attributes after one controlled reconnect. The current Android audio inventory separately showed USB-headset output and input endpoints. This supports the observed USB control/audio relationship, but does not prove listening quality, microphone recording or call routing. Only **one controlled reconnect** is complete; initial attached-state inventory is not counted as another controlled insertion.

## Button → Linux/HID → Android → action observations

Linux codes and scan values below are hexadecimal. Android names come from the actual selected layout, not numeric Linux-code substitution. The maintainer confirmed the three physical controls: Center, Volume Up and Volume Down.

| Physical gesture/context | Linux key / MSC_SCAN | Selected Android mapping | Evidence and observed action |
| --- | --- | --- | --- |
| Volume Up short presses, screen unlocked, media setup | KEY_VOLUMEUP `0x0073`; scan `0x000c00e9` | VOLUME_UP | 5 presses + 5 releases, no repeat value; user confirmed phone volume increased |
| Volume Down short presses, screen unlocked | KEY_VOLUMEDOWN `0x0072`; scan `0x000c00ea` | VOLUME_DOWN | 5 presses + 5 releases, no repeat value; user confirmed phone volume decreased |
| Center short-press window, media setup | KEY_PLAYPAUSE `0x00a4`; scan `0x000c00cd` | MEDIA_PLAY_PAUSE | 7 presses + 7 releases in a window requesting 5; user confirmed play/pause worked but did not confirm the actual physical count. Cause of extra recorded pairs unresolved |
| Sustained/repeat/double gestures; locked/off; calls | Not measured | Not established | Do not offer independent call/assistant/track semantics or claim these contexts passed |

An initial 45-second non-PTY capture produced zero event bytes despite a subsequent user report of pressing all three buttons five times, in Center → Volume Down → Volume Up order. Its expected timeout exit was 124; absence of data is an inconclusive capture, not proof of blocked or absent button signals. Both two-second verbosity probes returned device headers, with and without a PTY. Later PTY-backed recordings received events. The initial failure's cause is unresolved and remains in private evidence; it is not silently replaced by the later recordings.

Successful captures retained all scan/key/synchronization records. They contain 102 events and 34 complete SYN_REPORT frames, no outstanding pressed key at their ends, and no observed SYN_DROPPED. This validates the captured sequences only; it does not test overrun recovery or a filter implementation. The private diagnostic reader used preflight identity checks, a remote hard timeout, a host deadline, a 64 KiB limit and a visible Stop/Done interaction. Its short parser self-checks and host sequence assertions passed.

## Evidence and artifact identities

- Runtime module artifact: **none**. Every observation used source context `363bf13e10125cf75b19df7ef1fdd28340c1daec` with existing phone tools; no EarKey binary ran on the phone.
- Sanitized, relative-time [physical event fixture](../../tests/fixtures/discovery/reference-2026-11-04-short-presses.json): SHA-256 `22a4de40222f9ac26bba4962edabe260933010a4f3c563d801a126687fa38be2`. It is diagnostic test data, not a policy/profile or acceptance certificate.
- Volume Up raw event record SHA-256: `d6b99428b17c06201475c043214999f81e21822bf1964b342c8cd5fcaa85f638`; Volume Down/Center raw event record: `489a86ad43adb36de79d16d00ff6c0927caab8655c7f11557dec5d609fa316d0`.
- Private reader source SHA-256: `d789a894dea1b14dda82212d872138c42e5d6d26c161ab867127485c6f2cf22f`. This temporary diagnostic reader is not the production identity/filter implementation.
- Private checkpoint manifest SHA-256: `1b2cf8b506fe318bdab38dd6aaed8abce3b386a1c6938ec744767d8de270a86d`, covering 103 evidence files / 117,586 bytes before the manifest itself. Raw records are under ignored private storage with mode 0600 inside private directories. Transport identifiers, serial/unique IDs, unrestricted mapping/audio diagnostics and incidental logs are excluded from this report/fixture.

Source timestamps, exit/stop reasons and raw digests remain in the private records. Public fixture event timestamps are relative; collection UTC metadata is retained for traceability. Publishing this sanitized subset is a separate approval gate, because its model/build/accessory details still disclose the tested setup publicly. The attempted public Stage 1 issue was rejected by automatic approval review and was not created.

## Remaining Stage 1 gates and backend decision

**The mountless grab + uinput architecture remains a candidate, not an approved device backend.** Mechanical scoping looks promising, but complete identity stability, root/service permissions and preservation/semantic evidence are missing. No profile is enrolled and no automatic activation is eligible.

- Complete two additional controlled reconnect/insertion observations, then confirm stable attributes and grouping; boot stability remains unmeasured and no reboot is authorized.
- Resolve the Center count discrepancy and obtain five observations per relevant sustained/repeat/double gesture and context. Explain shared playback/call/assistant effects from actual measurements.
- Cover paused, locked/off and consenting/local incoming/ongoing calls; obtain baseline microphone recording, listening/call routing and plug/unplug behavior.
- Explicitly test phone power/volume/touch and an unrelated input device; capture a defined idle/battery baseline. Incidental normal screen use is not a completed preservation test.
- Obtain permitted read-only root/kernel/installed-module/context evidence. File presence or successful shell getevent is not proof of the future root service's uinput/evdev permissions.
- Make the explicit eligible/unsupported native-backend decision before dependent implementation. All module acceptance T01–T28, passthrough, suppression, crash/hang, recovery, installation and manager lifecycle tests remain Not run.

## Research provenance

Consulted on 4 November 2026: official [ADB manual/source](https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/user/adb.1.md) through Context7's AOSP ADB index, [AOSP getevent](https://source.android.com/docs/core/interaction/input/getevent), [AOSP key layouts](https://source.android.com/docs/core/interaction/input/key-layout-files), [AOSP SELinux](https://source.android.com/docs/security/features/selinux), and [KernelSU module guide](https://kernelsu.org/guide/module.html) through `/tiann/kernelsu`. Runtime command help was checked for the phone's getevent and Toybox 0.8.13-android timeout. Help exits 1/125 are retained as usage-query results.

Context7 did not index ReSukiSU. Its [official project introduction](https://resukisu.org/guide/introduce) was consulted; the old [upstream repository URL](https://github.com/ReSukiSU/ReSukiSU) currently redirects to Baka-SU/BakaSU. That moving upstream is not treated as the installed v4.2.0-rc3 API contract. A matching source revision and actual kernel/userspace/service behavior remain unresolved. No root-manager upgrade or replacement is proposed.
