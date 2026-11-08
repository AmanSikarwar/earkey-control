# Recovery outline

**Planning only. No recovery path has been exercised; no module or commands are implemented.** This document must become a device-specific tested guide before any interception prototype is installed. Do not use unimplemented command names as recovery instructions.

## Before a prototype

Record the exact root implementation/fork, manager, kernel and ROM, physical power/volume functionality, a verified way to disable only `earkey_control`, and a separate usable privileged debugging/recovery route if available. Verify USB debugging authorization, recovery access and data decryption capabilities only with permission. Do not assume encrypted `/data` is accessible or a bootloader/recovery offers a root shell.

Provide a durable module-specific next-boot inactive marker and check it before opening input nodes. Location is `/data/adb/earkey_control/stay-inactive`, with root-only ownership. The host store now validates the marker, creates or removes it with file and directory synchronization, and exposes generation-checked pause, boot-escape and restart-latch operations with synthetic tests. The root control surface, device behavior and the full recovery drill remain unmeasured. The temporary prototype also needs a finite session deadline and independent supervisor. Stage 6 integrates these protections; it does not defer them until after the prototype.

## Recovery layers and expected behavior

| Situation | Planned route and required outcome |
| --- | --- |
| Normal use / unwanted behavior | Pause now via UI or root command interface: release interception, clear forwarded keys where possible, destroy proxy, show actual inactive acknowledgment |
| UI unavailable | Manager-independent root control route using already validated access; do not require earphone buttons |
| Worker crash or hang | Independent supervisor stops verified worker instance, releases resources and records fault; after three failures latch inactive |
| Prevent next-boot activation | Set durable inactive state; boot checks it before candidate opens; preserve it across update/reboot |
| Manager disable | Disable only this module, verify actual immediate/next-boot timing; pause first when accessible |
| Uninstall | Stop only verified module processes, release grabs/proxies, remove only module-owned files; selected settings-retention behavior must be disclosed and tested |
| Android not usable | Apply exact installed manager's verified rescue procedure or prevalidated privileged disable route; no universal recipe is asserted |

KernelSU's official rescue guidance includes volume-down input with timing/integration qualifications; it is not proof for the user's unmeasured fork. Magisk and APatch have their own mechanisms. Consult and verify the installed implementation before an experiment:

- [KernelSU rescue](https://kernelsu.org/guide/rescue-from-bootloop.html)
- [Magisk FAQ and recovery](https://topjohnwu.github.io/Magisk/faq.html)
- [APatch rescue](https://apatch.dev/rescue-bootloop.html)

Some upstream fallbacks affect all modules. Prefer a verified module-specific disable path; do not turn broad removal guidance into the default product recovery. Do not wipe data, flash unrelated images, delete root-manager executables, replace firmware, or change the root manager/kernel as routine recovery. An out-of-scope recovery needs a concrete maintainer-approved device plan.

## Required drill

Test immediate pause while idle and while a key is held, crash, hang, output failure, unplug, restart-limit latch, boot escape, manager disable and uninstall. Verify allowed keys are not stuck, original phone and accessory input return, audio/microphone/routing remain functional, and no hidden service survives removal. Record actual pause/release time, artifact digest, commit, environment and evidence. T14, T16–T19, T23–T24 and T28 are unresolved physical gates.
