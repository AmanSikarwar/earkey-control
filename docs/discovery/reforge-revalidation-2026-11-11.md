# Reference kernel revalidation, 11 November 2026

The maintainer confirmed updating the reference phone's kernel. The alpha.4 upgrade stopped at the preflight kernel check. Read-only checks over ADB between 09:58 and 10:08 UTC found alpha.3 installed with its restart latch set. One module daemon was present, with no active input worker or EarKey Control proxy.

## Measured environment

| Field | Observation |
| --- | --- |
| Phone and ROM | Redmi Note 10 Pro `M2101K6P` / `sweetin`, `PixelOS_sweet-17.0-20261030-2116`, Android API 37, build `CP2A.260605.016` / `1793394994` |
| Kernel | `/proc/version` reports `4.14.357-reforge-gc216b1fe2466`, build `#2 SMP PREEMPT Tue Nov 10 05:21:32 UTC 2026` |
| Driver | `version=35205`, `full_version=v4.2.0-35205-backslashxx`, UAPI 6, flags `0x160`, features `0x7`, built in, `lkm=false`, `bundled=false`, `late_load=false`, `pr_build=false` |
| ABI and page size | `arm64-v8a`, 4096 bytes |
| Root environment | ReSukiSU `com.resukisu.resukisu`, `v4.2.0-rc3`, code 35257. The `ksud` binary and both Magic Mount RS lifecycle hook digests matched the prior review. The root shell and live module daemon were UID 0 in `u:r:ksu:s0`, with SELinux enforcing |
| WebView | `com.google.android.webview` `156.0.8078.47` |

The connected Xiaomi earphones matched the saved enrollment's serial in a private on-device comparison. One HID collection reported input ID `0003:0020:0b21:0201`, USB ID and revision `0020:0b21:0100`, interface `03` class `03` using `usbhid`, and the same event, key and scan capabilities as the [previous reforge measurement](reforge-revalidation-2026-11-10.md). Interfaces `00`, `01` and `02` still used `snd-usb-audio`. Android reported `KEYBOARD | EXTERNAL`, selected `Generic.kl` and `Generic.kcm` with no device configuration override, and both file digests matched the earlier record. Gesture behavior and audio or microphone preservation still need physical checks on the updated kernel.

## Stage 6.6 checkpoint

The planned upgrade targeted the reviewed alpha.4 ZIP from `f2fc01375dd5ab46bc09c95b86a3a414338c0119`, SHA-256 `be8ddda5a07db37b5b4e9e9627d7412a5c8d9c73e29393c59317423833147f9c`. The expected baseline was alpha.2 on kernel `g9b21e3df6c0a`. The observed kernel and installed module both differed. There was no pending stage, update or remove operation at the follow-up. The run ended before pause, upload, install or reboot.

The installed alpha.3 worker matched `284ecfeb0405598a254f018d2ee17ad6ce79f873a58f73d5bc5ceb509bbd2b21` from source `47f7e58890390488a9f645813749027d6272cd94`. All four installed WebUI assets matched the locally reproduced alpha.3 package from `4e9aaf720b88edbb2909a70f4e5b99708f1190c0`, ZIP SHA-256 `cf29c568f950059593098bba47b1c18005073a6d8e0ed6f81431ba7f4c8a4f50`.

| Case | Expected and observed | Result |
| --- | --- | --- |
| T23 upgrade preflight | Exact earlier kernel and installed alpha.2 required. The phone reported the updated kernel and alpha.3 | Stopped at preflight. Upgrade and settings retention across it remain pending |
| T25 changed kernel | Existing profile should refuse the new environment. The installed worker's read-only `check` returned `evidence=none`, `check=fail`, exit 1 before evdev acquisition | Safe refusal observed. The new combination is Unsupported for interception by these artifacts |
| T27 read-only status | The manager page should show the saved restart latch. The maintainer opened it and reported Fault and “stopped after repeated failures.” Root status showed `Latched`, saved generation 5 and active generation 0. Policy and enrollment digests matched before and after, with no worker or proxy | Status bridge check passed on alpha.3. Settings changes, Pause now, TalkBack, 200% text and offline operation on Android remain pending |

The saved selection covers all three measured controls. The current daemon reported zero failures and no last fault in this run, while the persistent latch was set. Its earlier cause remains unestablished. The latch and store were left intact. The serial comparison ran on the phone. The comparison result, private digests and device records stay in the ignored evidence directory.

Prepare an exact profile for this kernel and verify acquisition, proxy behavior and supervised physical passthrough before using it for suppression. Use the observed alpha.3 installation and saved recovery state for the next upgrade plan. [Stage 6 plan](../stage-6-plan.md), [compatibility](../compatibility.md)
