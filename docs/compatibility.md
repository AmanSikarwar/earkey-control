# Compatibility contract

Status as of 6 November 2026: **no physically qualified module combinations; tested support minimum/maximum: none**. Read-only Stage 1 baseline measurements are recorded in the [discovery checkpoint](discovery/reference-2026-11-04.md), including approved screen-off volume/hold observations and later local Center double-attribution/identity, ongoing-call volume and paused-Center/volume follow-ups approved for publication on 6 November at `1be76a95d6c5ffe143c11d33464169f7e1f85118`. Five individually reset Center holds have user-reported wake/assistant activation; the later five double groups have one reported track change each, a dark screen and no assistant activation. One completed ongoing cellular-call volume window has five physical presses per button, reported volume changes and user-confirmed uninterrupted, clear two-way audio. A paused-Center double baseline adds one-plus-four groups with track-change reports; the later four have explicit one-change-each and paused-continuation confirmation. Five separately reset paused-start Center holds now have reported assistant activation and explicit confirmation that music stayed paused after assistant listening ended. Earlier mixed/interrupted attempts remain separate. No installation/interception has been attempted. Baseline discovery does not qualify module support.

The later paused-volume hold baseline has five user-confirmed physical holds per button, with paused playback during the volume-only portions. Up's reported increase conflicts with stored indices 9→7; Down's 15→7 endpoint includes later accidental Center/Up input. Both limits remain unresolved for quantitative comparison, and neither changes the Design target support level.

| Level | Definition |
| --- | --- |
| Verified | Named combination passed the required physical suite on the released artifact or an identical build with recorded digest |
| Experimental | Some evidence exists; full physical acceptance is incomplete; activation requires an explicit experimental choice |
| Design target | Implementation/tests planned; no working-support claim |
| Unsupported | Prerequisite missing, identity ambiguous, or preservation unestablished for the evaluated combination; do not intercept; explain reason |

An untested roadmap row remains Design target; an evaluated combination that fails a prerequisite becomes Unsupported. Installation or an accepted ZIP alone never promotes a support level. Host tests and emulators cannot promote an accessory to Verified.

The reference Stage 1 Center answer/end repetition item is closed on maintainer verification under [D10](project-contract.md#reference-center-call-validation-amendment--5-november-2026). One instrumented trial remains the captured evidence; this closure adds no module acceptance or support qualification.

The maintainer reports that holding inline volume buttons during ongoing calls changed only the volume while held. This completes the Stage 1 follow-up for that gesture on maintainer verification. Compatibility remains Design target and module call acceptance is pending. [D11](project-contract.md#reference-ongoing-call-volume-hold-amendment--7-november-2026)

The maintainer reports that double-pressing an inline volume button during a call only displayed the volume slider. The Stage 1 follow-up for that gesture is complete on maintainer verification. [D12](project-contract.md#reference-ongoing-call-volume-double-press-amendment--7-november-2026)

The maintainer reports that holding Center during an ongoing call toggled mute. The Stage 1 follow-up for that gesture is complete on maintainer verification. Microphone-path and module call acceptance still need physical evidence. [D13](project-contract.md#reference-ongoing-call-center-hold-amendment--7-november-2026)

The maintainer reports that pressing Center ended an ongoing call. This closes the planned Stage 1 double-press follow-up at their request. The number of taps and a distinct double-press action remain unverified. [D14](project-contract.md#reference-ongoing-call-center-double-press-amendment--7-november-2026)

## Initial matrix

| Entry | Android/API, ROM, kernel | Root implementation / manager | ABI / page size | Accessory / transport | Backend / groups | Level / evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Reference Redmi Note 10 Pro | M2101K6P/sweetin; PixelOS_sweet-17.0-20261030-2116; Android 17/API 37 reported; 4.14.357-openela-VantomKernel | ReSukiSU manager code 35257; userspace 4.2.0-rc3-86-g516d3e87/code 35257; kernel code 35240, UAPI 6, built-in; root shell ksu context; actual module service untested | arm64 advertised; 4 KiB pages; no ELF load test | USB HID interface 1.3 separate from audio interfaces; input IDs 0003:0020:0b21:0201; one measured reconnect, further repetition waived | Grab + uinput candidate; no proxy/grab. Center short/hold/double and one cellular answer/end use the same signal; listening, USB-mic and phone controls have qualitative baselines; Dozing/locked/off short keys and one instrumented idle sample measured; inline holds/doubles measured; paused media index stayed unchanged in paired checks; full context/isolation evidence pending | Design target with partial baseline discovery; no module support verified |
| Magisk expansion | Device, ROM, Android and kernel pending | Versions pending | Unmeasured | Pending | Same core candidate, root command interface | Design target; no test environment |
| APatch expansion | Device, ROM, Android and kernel pending | Versions pending | Unmeasured | Pending | Adapter and configuration path untested | Design target; no test environment |
| Android minimum | Android 12 / API 31 intended; no measured ROM/kernel | Each manager separately | Pending | Pending | Pending | Design target; no tested minimum |
| Newest Android release claimed | Must identify then physically test before claim | Each implementation separately | 4 KiB/16 KiB only where tested | Identified accessory required | Pending | Design target; no release selected |

For each actual result record device variant, Android/API, full ROM build, kernel build/release, root implementation/fork and kernel/userspace/manager versions, ABI, page size, accessory identity scope, transport/collections, backend, independent groups/shared effects, source commit, artifact SHA-256, evidence date/location, and T01–T28 outcomes. Record oldest/newest tested releases and explicit intermediate coverage. A single reference row does not cover other ROMs or forks.

Revalidate after ROM/kernel/root-manager updates, changed capabilities, layout digest changes, or profile evidence expiration. Downgrade stale claims when they cannot be reproduced. Separate all-control-only, mounted, framework, analog, additional digital-headset and USB-adapter backends/profiles. Do not infer them from the Type-C connector or another accessory. Do not change the user's kernel/root method to fill coverage.
