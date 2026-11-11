# Compatibility contract

The reforge `g9b21e3df6c0a` setup has **Experimental** evidence from the alpha.1 and alpha.2 phone runs. On 11 November the phone was found on reforge `gc216b1fe2466`, which the installed worker refuses. The verified support range is still empty. [Alpha device evidence](stage-6-plan.md#alpha2-boot-check-11-november-2026), [updated kernel revalidation](discovery/reforge-revalidation-2026-11-11.md)

The Vantom kernel's paused-volume hold baseline has five user-confirmed physical holds per button, with paused playback during the volume-only portions. Up's reported increase conflicts with stored indices 9→7. Down's 15→7 endpoint includes later accidental Center/Up input. Both limits remain unresolved for quantitative comparison.

| Level | Definition |
| --- | --- |
| Verified | Named combination passed the required physical suite on the released artifact or an identical build with recorded digest |
| Experimental | Some evidence exists; full physical acceptance is incomplete; activation requires an explicit experimental choice |
| Design target | Implementation/tests planned; no working-support claim |
| Unsupported | Prerequisite missing, identity ambiguous, or preservation unestablished for the evaluated combination; do not intercept; explain reason |

An untested roadmap row remains Design target; an evaluated combination that fails a prerequisite becomes Unsupported. Installation or an accepted ZIP alone never promotes a support level. Host tests and emulators cannot promote an accessory to Verified.

The reference Stage 1 Center answer/end repetition item is closed on maintainer verification under [D10](project-contract.md#reference-center-call-validation-amendment--5-november-2026). One instrumented trial remains the captured evidence; this closure adds no module acceptance or support qualification.

The maintainer reports that holding inline volume buttons during ongoing calls changed only the volume while held. This completes the Stage 1 follow-up for that gesture on maintainer verification. Module call acceptance is pending. [D11](project-contract.md#reference-ongoing-call-volume-hold-amendment--7-november-2026)

The maintainer reports that double-pressing an inline volume button during a call only displayed the volume slider. The Stage 1 follow-up for that gesture is complete on maintainer verification. [D12](project-contract.md#reference-ongoing-call-volume-double-press-amendment--7-november-2026)

The maintainer reports that holding Center during an ongoing call toggled mute. The Stage 1 follow-up for that gesture is complete on maintainer verification. Microphone-path and module call acceptance still need physical evidence. [D13](project-contract.md#reference-ongoing-call-center-hold-amendment--7-november-2026)

The maintainer reports that pressing Center ended an ongoing call. This closes the planned Stage 1 double-press follow-up at their request. The number of taps and a distinct double-press action remain unverified. [D14](project-contract.md#reference-ongoing-call-center-double-press-amendment--7-november-2026)

The maintainer reports that holding Center while an incoming call was ringing rejected it. The Stage 1 follow-up for that gesture is complete on maintainer verification. [D15](project-contract.md#reference-incoming-call-center-hold-amendment--7-november-2026)

The maintainer reports that double-pressing Center rejected a ringing call. The Stage 1 follow-up for that gesture is complete on maintainer verification. Per-tap behavior remains unmeasured. [D16](project-contract.md#reference-incoming-call-center-double-press-amendment--7-november-2026)

The maintainer reports that holding either inline volume button during an incoming call silenced the ringtone. The Stage 1 follow-up is complete on maintainer verification. Volume indices and call state remain unmeasured. [D17](project-contract.md#reference-incoming-call-volume-hold-amendment--7-november-2026)

The maintainer reports that double-pressing an inline volume button during an incoming call silenced the ringtone and showed the slider. The Stage 1 follow-up is complete on maintainer verification. Per-tap effects and call state remain unmeasured. [D18](project-contract.md#reference-incoming-call-volume-double-press-amendment--7-november-2026)

## Reference and design target matrix

| Entry | Android/API, ROM, kernel | Root implementation / manager | ABI / page size | Accessory / transport | Backend / groups | Level / evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Reference Redmi Note 10 Pro | M2101K6P/sweetin; PixelOS_sweet-17.0-20261030-2116; Android 17/API 37 reported; 4.14.357-openela-VantomKernel | ReSukiSU manager code 35257; userspace 4.2.0-rc3-86-g516d3e87/code 35257; kernel code 35240, UAPI 6, built-in; root shell ksu context; actual module service untested | arm64 advertised; 4 KiB pages; no ELF load test | USB HID interface 1.3 separate from audio interfaces; input IDs 0003:0020:0b21:0201; one measured reconnect, further repetition waived | Grab + uinput candidate; no proxy/grab. Center short/hold/double and one cellular answer/end use the same signal; listening, USB-mic and phone controls have qualitative baselines; Dozing/locked/off short keys and one instrumented idle sample measured; inline holds/doubles measured; paused media index stayed unchanged in paired checks; full context/isolation evidence pending | Design target with partial baseline discovery; no module support verified |
| Reference Redmi Note 10 Pro with reforge | M2101K6P/sweetin, PixelOS_sweet-17.0-20261030-2116, API 37, `4.14.357-reforge-g9b21e3df6c0a` | ReSukiSU manager and userspace code 35257, userspace revision `516d3e87`, built-in driver 35201 with UAPI 6 observed after boot, `u:r:ksu:s0`, enforcing | arm64, 4 KiB pages | Same USB HID collection and selected layout, revalidated under `xiaomi_pixelos_reforge20261110` | evdev grab with uinput. Center, Volume Up and Volume Down are the three measured groups | Experimental with explicit enrollment and activation. Alpha.1 passed basic blocking, pause and removal with one boot stall. Alpha.2 passed the recorded connected and absent boot checks. Full preservation, update, rescue and alpha.4 device checks are pending. [Results](stage-6-plan.md) |
| Reference Redmi Note 10 Pro with updated reforge | M2101K6P/sweetin, PixelOS_sweet-17.0-20261030-2116, API 37, `4.14.357-reforge-gc216b1fe2466` | ReSukiSU code 35257 with matched userspace digest, built-in driver 35205 with UAPI 6, `u:r:ksu:s0`, enforcing | arm64, 4 KiB pages | Same enrolled Xiaomi USB HID collection and Android layout | Inactive. Current artifacts have no profile for this exact kernel | Unsupported for interception by the current artifacts. Alpha.3's read-only check rejected the kernel, and its manager page displayed the saved latch. [Results](discovery/reforge-revalidation-2026-11-11.md) |
| Magisk expansion | Device, ROM, Android and kernel pending | Versions pending | Unmeasured | Pending | Same core candidate, root command interface | Design target; no test environment |
| APatch expansion | Device, ROM, Android and kernel pending | Versions pending | Unmeasured | Pending | Adapter and configuration path untested | Design target; no test environment |
| Android minimum | Android 12 / API 31 intended; no measured ROM/kernel | Each manager separately | Pending | Pending | Pending | Design target; no tested minimum |
| Newest Android release claimed | Must identify then physically test before claim | Each implementation separately | 4 KiB/16 KiB only where tested | Identified accessory required | Pending | Design target; no release selected |

For each actual result record device variant, Android/API, full ROM build, kernel build/release, root implementation/fork and kernel/userspace/manager versions, ABI, page size, accessory identity scope, transport/collections, backend, independent groups/shared effects, source commit, artifact SHA-256, evidence date/location, and T01–T28 outcomes. Record oldest/newest tested releases and explicit intermediate coverage. A single reference row does not cover other ROMs or forks.

The maintainer installed the reforge kernel deliberately. The measured build is `#2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026`. The Stage 4 revalidation matched the accessory identity, capabilities and selected layout. In the measured service domain, the v9 probe loaded, v10 opened `/dev/uinput`, v12 opened and revalidated the evdev node, and v14 verified Android's proxy classification. Later diagnostics exercised physical passthrough and suppression. These records apply to their exact diagnostic artifacts and the reforge environment. [Revalidation](discovery/reforge-revalidation-2026-11-10.md), [diagnostic results](prototype-preflight.md)

Revalidate after ROM/kernel/root-manager updates, changed capabilities, layout digest changes, or profile evidence expiration. Downgrade stale claims when they cannot be reproduced. Separate all-control-only, mounted, framework, analog, additional digital-headset and USB-adapter backends/profiles. Do not infer them from the Type-C connector or another accessory. Do not change the user's kernel/root method to fill coverage.
