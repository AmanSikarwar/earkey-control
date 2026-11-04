# Compatibility contract

Status as of 4 November 2026: **no physically qualified module combinations; tested support minimum/maximum: none**. Read-only Stage 1 baseline measurements exist in a local [discovery checkpoint](discovery/reference-2026-11-04.md), whose publication awaits approval. No installation/interception has been attempted. Baseline discovery does not qualify module support.

| Level | Definition |
| --- | --- |
| Verified | Named combination passed the required physical suite on the released artifact or an identical build with recorded digest |
| Experimental | Some evidence exists; full physical acceptance is incomplete; activation requires an explicit experimental choice |
| Design target | Implementation/tests planned; no working-support claim |
| Unsupported | Prerequisite missing, identity ambiguous, or preservation unestablished for the evaluated combination; do not intercept; explain reason |

An untested roadmap row remains Design target; an evaluated combination that fails a prerequisite becomes Unsupported. Installation or an accepted ZIP alone never promotes a support level. Host tests and emulators cannot promote an accessory to Verified.

## Initial matrix

| Entry | Android/API, ROM, kernel | Root implementation / manager | ABI / page size | Accessory / transport | Backend / groups | Level / evidence |
| --- | --- | --- | --- | --- | --- | --- |
| Reference Redmi Note 10 Pro | M2101K6P/sweetin; PixelOS_sweet-17.0-20261030-2116; Android 17/API 37 reported; 4.14.357-openela-VantomKernel | ReSukiSU manager v4.2.0-rc3/35257 installed; active root kernel/userspace/service context unverified | arm64 advertised; 4 KiB pages; no ELF load test | USB HID candidate, input IDs 0003:0020:0b21:0201, interface 1.3; identity stability incomplete | Grab + uinput candidate only; short volume/play-pause observations, contextual semantics pending | Design target with partial baseline discovery; no module support verified |
| Magisk expansion | Device, ROM, Android and kernel pending | Versions pending | Unmeasured | Pending | Same core candidate, root command interface | Design target; no test environment |
| APatch expansion | Device, ROM, Android and kernel pending | Versions pending | Unmeasured | Pending | Adapter and configuration path untested | Design target; no test environment |
| Android minimum | Android 12 / API 31 intended; no measured ROM/kernel | Each manager separately | Pending | Pending | Pending | Design target; no tested minimum |
| Newest Android release claimed | Must identify then physically test before claim | Each implementation separately | 4 KiB/16 KiB only where tested | Identified accessory required | Pending | Design target; no release selected |

For each actual result record device variant, Android/API, full ROM build, kernel build/release, root implementation/fork and kernel/userspace/manager versions, ABI, page size, accessory identity scope, transport/collections, backend, independent groups/shared effects, source commit, artifact SHA-256, evidence date/location, and T01–T28 outcomes. Record oldest/newest tested releases and explicit intermediate coverage. A single reference row does not cover other ROMs or forks.

Revalidate after ROM/kernel/root-manager updates, changed capabilities, layout digest changes, or profile evidence expiration. Downgrade stale claims when they cannot be reproduced. Separate all-control-only, mounted, framework, analog, additional digital-headset and USB-adapter backends/profiles. Do not infer them from the Type-C connector or another accessory. Do not change the user's kernel/root method to fill coverage.
