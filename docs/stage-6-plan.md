# Stage 6 plan — KernelSU integration

Date: 11 November 2026. Stage 6 turns the Stage 4 and 5 diagnostics into the installable `earkey_control` module. The backlog exit criterion is unchanged: an experimental release candidate with complete reference-phone evidence. It covers boot with and without the accessory, reconnect, live change, update, disable, uninstall and rescue, and mountless operation without a metamodule where supported.

| Increment | Scope | State |
| --- | --- | --- |
| 6.1 Root control commands | `enroll`, `status`, `apply <selection> [enable]`, `enable`, `disable`, `pause`, `resume`, `disable-next-boot`, `enable-next-boot` over the private store. Enrollment only from a unique live observation, policy writes that keep recovery state, and status without serials | implemented, host-tested |
| 6.2 Production worker loop | `earkey-worker run`: load enrollments and the verified policy each pass. Wait without failing while disabled, recovery-inactive, unreadable, accessory-absent, not neutral or grab-conflicted. Forward with the policy's blocked groups, reacquire on a new saved generation, and return to waiting on unplug | implemented, end-to-end container test |
| 6.3 Supervisor daemon | `earkey-worker daemon`: unbounded supervision of `run` with the real persisted restart latch, `clear-latch` to resume, `worker.status`/`supervisor.status` runtime files, and `status` reporting saved and active generations separately | implemented, container-tested |
| 6.4 Module package | `module/` plus `tools/package-module.py`: KernelSU boot-mode, arm64 and API 31 installer checks; a 0700 private store created or kept; `service.sh` starts the daemon only for the pinned digest; `uninstall.sh` stops only this module's processes and removes the store; runtime license files bundled; inactive after a fresh install | 0.1.0-alpha.1 install plan passed on the reference phone; 0.1.0-alpha.2 (readiness-progress fix) passed its boot check on the reference phone |
| 6.5 Offline WebUI | `webui/` (TypeScript 6.0.3): state, recovery, Pause now, master switch, Block all and measured Custom groups. Fixed operations use the synchronous `ksu.exec` bridge under a self-only CSP | Alpha.3 status displayed the saved latch in the installed manager. Alpha.4 has host evidence for confirmed policy and pause status |
| 6.6 Reference evidence | Validate the upgrade, settings retention, a held-key policy change and immediate pause | The installed alpha.3 refused the updated kernel. The updated-kernel passthrough executable is prepared for an authorized phone run. [Read-only revalidation](discovery/reforge-revalidation-2026-11-11.md), [test plan](#updated-kernel-passthrough-plan) |

Each increment goes through its own PR. Phone runs need separate maintainer authorization.

## Module package 0.1.0-alpha.3

The package is built by `tools/package-module.py` from the pinned worker. Two clean builds from `47f7e58` gave identical 928,408-byte ELFs, SHA-256 `284ecfeb0405598a254f018d2ee17ad6ce79f873a58f73d5bc5ceb509bbd2b21`. They pass the NDK pins and ELF checks: API 31, four `LOAD` segments aligned to `0x4000`, only `libc`/`libdl`, all 93 imports in API 31, and no host paths. The ELF is kept at `target/stage6/47f7e58/earkey-worker-arm64-api31`. The ZIP has `module.prop` at its root, the scripts, `skip_mount`, `bin/earkey-worker` with its digest, the compiled WebUI under `webroot/`, `LICENSE`, `THIRD_PARTY_NOTICES.md`, and `licenses/<crate>-<version>/` for every crate in the Android dependency tree. Two builds of `earkey-control-0.1.0-alpha.3.zip` were byte-identical: 535,352 bytes, SHA-256 `cf29c568f950059593098bba47b1c18005073a6d8e0ed6f81431ba7f4c8a4f50`.

Alpha.3 adds the WebUI. `status` now also prints one `control=<name>:<effect>|...` line for each measured control of each enrolled accessory, and the WebUI offers only those controls. Alpha.2 (`05371b5e…41ad`, worker `2179496b…4dca`) is the build that passed the boot check below.

Alpha.2 differs from alpha.1 in two ways. The worker now sends progress frames while it waits for `system_server` to open the proxy, so a slow early boot no longer trips the 1 s progress deadline. The installer now prints the installed worker path, not the `modules_update` staging path.

**Install behavior:**
- `customize.sh` requires the boot-mode KernelSU installer (`BOOTMODE=true`, `KSU=true`), arm64 and API 31 or later.
- It creates `/data/adb/earkey_control` as `0:0` `0700`, or keeps an existing one, so updates preserve settings. It refuses a symlinked store.
- With no policy, the policy is inactive and disabled, so the daemon starts and the worker waits as `disabled` without opening any device.

**Service and uninstall:**
- `service.sh` starts `earkey-worker daemon` in the background, only if the binary matches its digest and the store is a real directory.
- `uninstall.sh` runs when the manager prunes the module (before the services stage at boot), or during a full KernelSU uninstall. It stops only processes whose executable is this module's binary (`daemon` with SIGTERM, a 5 s bound, then SIGKILL; then any `run`). It then **removes the private store, including the policy, the enrollment serial and the runtime status**. Keeping settings across an uninstall is not offered yet.

**WebUI:** open the module in the KernelSU manager. The page runs only the fixed `earkey-worker` operations: `status`, `enroll`, `enable`, `disable`, `pause`, `resume`, `disable-next-boot`, `enable-next-boot`, `clear-latch`, and `apply` with `block-all`, `passthrough` or a list of measured group names. It calls the manager's synchronous `ksu.exec(command)` bridge with a 15 s `timeout`. Status refreshes every 3 s while the page is visible. Device text is rendered only as text, and the CSP allows only the page's own scripts and styles. The bridge comes from the KernelSU manager source (`WebViewInterface.kt`, `exec(cmd)`). That ReSukiSU exposes the same interface is unverified until the phone run.

**Use, as root, with `M=/data/adb/modules/earkey_control/bin/earkey-worker`:**
1. `$M enroll` with the earphones connected.
2. `$M apply block-all enable`, or `$M apply center enable`, and so on.
3. `$M status` reports the saved and active generations.

Stopping without the earphone buttons: `$M pause` or `$M disable-next-boot`. After repeated faults: `$M clear-latch`.

**Install plan (each run needs authorization):**
1. Install the ZIP with `ksud module install`.
2. Reboot and confirm the daemon runs, with `worker.status` showing `disabled`.
3. `enroll`, then `apply block-all enable`. Confirm `active_generation` matches and that the maintainer sees the earphone buttons blocked and the phone keys working.
4. `apply center enable` live; confirm only Center is blocked.
5. `pause`, then `resume`.
6. Unplug, then replug.
7. Reboot with the earphones connected and confirm blocking resumes.
8. Mark the module for removal, reboot, and confirm the store and processes are gone.

## First install results (0.1.0-alpha.1, 11 November 2026)

This was an authorized run on the reference phone with the reforge kernel `4.14.357-reforge-g9b21e3df6c0a`, using ZIP `4d515ea2bd4d9b5a7a49a5aa40478ea9408e546bdf222c394ef1869970300955` and worker `3327239e45c7179e302fda9023cd784348277006c9537daec9002631f441d26c`. The maintainer reported the button results; the phone reported the states. Raw logs stay private.

| Step | Result |
| --- | --- |
| 1. Install | Staged correctly with the pinned digest |
| 2. Reboot | `daemon` and `run` running; `worker.status` `disabled`; inactive |
| 3. Enroll, block all | `enrolled:xiaomi_pixelos_reforge20261110`; generation 1 active with all three groups; the maintainer pressed every control and none acted |
| 4. Center live | Generation 2 active without a restart; the maintainer confirmed only Center was blocked |
| 5. Pause, resume | The proxy was gone 0.66 s after `pause` (`Inactive(Paused)`) and Center worked while paused; `resume` activated generation 4 |
| 6. Unplug, replug | The worker returned to waiting and reacquired the earphones; the maintainer confirmed the expected behavior |
| 7. Reboot connected | Generation 4 active, Center blocked and Volume Up working, as confirmed by the maintainer. The supervisor recorded one `ProgressStalled` failure (see below) |
| 8. Remove | After marking for removal and rebooting: module directory, staged update, private store, all `earkey-worker` processes and the proxy are gone. The other six modules, the `ksud` digest, the metamodule link and enforcing SELinux are unchanged |

The first removal attempt did not run because wireless ADB was offline; the retry ran and passed.

Step 7's failure came from the worker loop sending no progress frames while it waited for `system_server` to open the proxy. Early in boot that wait can exceed the 1 s progress deadline. One such failure only restarts the worker, but three in a row would latch blocking off. Alpha.2 fixes this, and the run test now holds Android back for 1.6 s and requires progress throughout.

## Alpha.2 boot check (11 November 2026)

This was an authorized run with the same phone and kernel, using ZIP `05371b5eb6a8fbb4d9f3db8415e527d4fe7d42ce2060830ff91db3fc26ec41ad` and worker `2179496b26d866d9b73c824658db45ee2961b9db58276aca81b83ac9f8f74dca`. The store was empty, because the alpha.1 removal had deleted it.

| Step | Result |
| --- | --- |
| Install | Staged with the pinned digest; the installer printed `/data/adb/modules/earkey_control/bin/earkey-worker` |
| Reboot | Inactive: `worker.status` `disabled`; supervisor `failures=0` |
| Enroll, Center | `enrolled:xiaomi_pixelos_reforge20261110`; generation 1 active with Center blocked |
| Reboot, earphones absent | The maintainer plugged them in only after boot. The worker waited as `waiting_accessory` and acquired them on plug-in without restarting; `failures=0` |
| Reboot, earphones connected | Generation 1 active at 87 s uptime with the original `run` process; supervisor `failures=0`, `last_fault=none`. The maintainer confirmed that Center did nothing and Volume Up worked. SELinux is enforcing |

The `ProgressStalled` failure seen with alpha.1 did not recur on that boot. At the end of the run, alpha.2 remained installed with Center blocked.

## Alpha.4 upgrade and WebUI check — 11 November 2026

The alpha.3 review found that the worker wrote `active_generation` before the exclusive grab and that a settings change could end a forwarded press before its physical release. The WebUI also displayed a generic success on save and showed Pause as complete before the worker stopped. Alpha.4 preserves the old policy through the held press, waits for neutral keys and a complete frame before reacquiring, and publishes the new generation after forwarding starts. Pause still requests immediate stop. The WebUI displays the last request, saved and reported active generations and waits for the supervisor to confirm release. The installer leaves the WebUI permissions set by the manager.

The measured ReSukiSU userspace revision is `516d3e87fbc986ccefdde2c2765f1700612441a9`. Source reviewed on 11 November exposes `ksu.exec(cmd)` and serves the bundled page through `WebViewAssetLoader`. The installed manager APK and Android WebView still need the device check. [Bridge source](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/manager/app/src/main/java/org/bakasu/bakasu/ui/webui/WebViewInterface.kt#L23-L33), [WebView setup](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/manager/app/src/main/java/org/bakasu/bakasu/ui/webui/WebViewHelper.kt#L97-L141), [installer source](https://github.com/Baka-SU/BakaSU/blob/516d3e87fbc986ccefdde2c2765f1700612441a9/userspace/ksud/src/android/module/installer.sh#L415-L430), [KernelSU WebUI guide](https://kernelsu.org/guide/module-webui.html)

| Artifact | Source | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| Alpha.4 worker (`target/stage6/9bda4e8/earkey-worker-arm64-api31`) | `9bda4e85bdb6252ad61e219c772699ff68f3fe3c` | 932,288 | `f49b14645a0984e200b4d531d0672a11b953447234bdd68fa539eabe3462b571` |
| `earkey-control-0.1.0-alpha.4.zip` (`target/stage6/f2fc013/`) | `f2fc01375dd5ab46bc09c95b86a3a414338c0119` | 537,472 | `be8ddda5a07db37b5b4e9e9627d7412a5c8d9c73e29393c59317423833147f9c` |
| Reproduced alpha.2 ZIP for rollback (`target/stage6/dbd3db0/`) | `22d03c13080635abb344325575f69f7d1a7f5512` | 526,994 | `05371b5eb6a8fbb4d9f3db8415e527d4fe7d42ce2060830ff91db3fc26ec41ad` |

Two clean builds matched for the alpha.4 ELF and ZIP. Rust 1.99.1 and NDK r28c produced an AArch64 API 31 PIE with four 16 KiB aligned load segments and 93 imports present in the API 31 stubs. TypeScript 6.0.3, Node 26.13.0, npm 11.20.0 and Python 3.15.2 produced the ZIP. Its 53 entries passed CRC, inventory, version, mode, line-ending and digest checks. See the [build recipe](build.md#stage-6-module-package).

Host checks passed on the pinned arm64 LinuxKit `7.0.14-linuxkit` container (125 Rust tests, including synthetic uinput grab conflict, held-key reload and immediate stop), and on macOS (5 WebUI tests and 8 installer/package tests, Rust formatting, Android-target Clippy, dash syntax and BusyBox ShellCheck). Chrome `157.0.8103.39` exercised the exact packaged page with a simulated bridge in a 360 px viewport. After loading the bundled assets it passed the save and pause transitions in offline mode, reflowed at 200% root text size, and exposed labeled controls and visible Tab focus. Actual bridge execution, TalkBack and Android text scaling are pending.

### Original reference run plan

Use the measured [reforge environment](discovery/reforge-revalidation-2026-11-10.md). The last recorded phone state has alpha.2 installed with Center blocked. Confirm that state before proceeding. This scope covers one alpha.4 update using the normal ReSukiSU installer and its previously reviewed delegation, one reboot, bounded module status reads, and the checks below ending paused. Record UTC time, the exact environment and ZIP/ELF digests, expected and observed outcomes, and evidence location. Keep policy contents, serials, and any audio private.

| Case | Expected evidence |
| --- | --- |
| T23 preflight and stage | Check the installed alpha.2 digest, manager/hook digests, SELinux, and absence of pending module operations. Pause alpha.2 and verify release before staging. Record private policy/enrollment hashes and the saved generation after pausing. Verify the alpha.4 ZIP digest before installation and the staged metadata and worker digest before reboot. Remove the temporary ZIP upload |
| T23 reboot and retention | After one reboot, the installed worker digest, `versionCode=4`, policy/enrollment hashes, and saved pause state match. One daemon waits in `Inactive(Paused)`, with no worker grab or proxy |
| T27 actual WebUI | Open EarKey Control in the installed manager. Read the measured profile, saved generation and paused state. Check the group labels and linked Center effects, keyboard focus, TalkBack and 200% Android text scaling. Test with Internet unavailable if the independent recovery route remains available |
| T05/T15 activation and held key | Resume the retained Center-only selection and confirm playback, volume and the phone keys. While an allowed inline volume press is held, save Block all. The old generation stays active through release. The new generation appears only after reacquisition. Confirm the newly blocked key has no action on its next press |
| T24 immediate pause | Pause from the WebUI and check the supervisor confirms inactivity, the proxy is gone, and the inline controls work normally. Record time from request to observed release. Restore the original selection while paused and leave the module paused |

Stop the experiment on an identity or digest mismatch, an unexpected module operation, or a preservation failure. Use the already available root `pause` command if the page fails, then `disable-next-boot` if the service cannot confirm release. For a rollback, use the digest-verified alpha.2 ZIP with the same v1 store, after review of the observed failure and the rollback installation and reboot scope. The rollback ZIP was reproduced on the host. An actual downgrade remains pending. [Recovery](recovery.md)

Record the release and reacquisition interval because physical actions can pass through between sessions. Confirm live process and proxy state alongside the UI. A sudden daemon exit can leave a status file behind, which is a Stage 7 fault case.

This run covers the update and UI increment. The outstanding physical gates include call and microphone preservation, unrelated input, manager disable and independent rescue, a mountless run without a metamodule where supported, and separate upstream KernelSU evidence. Retain the earlier reconnect evidence and the reference repetition waiver. [Validation matrix](validation-plan.md#physical-t01t28-matrix)

### Preflight result after kernel update

The phone now runs reforge `gc216b1fe2466` with driver 35205 and has alpha.3 installed. The approved alpha.4 run stopped before policy change, staging or reboot. The installed worker refused the updated kernel in its read-only `check`. The maintainer confirmed that the real alpha.3 WebUI showed Fault and the restart-latch reason, and the policy and enrollment digests were unchanged afterwards. The next device plan must bind the new kernel and preserve the observed alpha.3 recovery state. [Measured revalidation and case outcomes](discovery/reforge-revalidation-2026-11-11.md)

## Updated-kernel passthrough plan

The opt-in `reforge-passthrough` build adds an in-memory profile for the exact 11 November ROM and kernel build. The stored enrollment from the previous reforge build supplies the reference serial privately. Acquisition requires a unique match with the measured USB HID collection, driver, full capability set and layout. It rechecks identity on the opened descriptor against a refreshed inventory and requires neutral input. The new evidence is rejected by the persistent policy and enrollment paths. The alpha.4 package retains its existing worker pin. [Measured environment](discovery/reforge-revalidation-2026-11-11.md#measured-environment)

The temporary root-shell executable uses the existing worker and supervisor. It requires the observed enforcing `u:r:ksu:s0` context, 4 KiB pages and the installed module's latched state. The installed policy, enrollment and runtime files are read only. The diagnostic accepts `check`, `proxy-check` and `supervise`. The internal worker requires the same executable as its parent. The session forwards every measured control, with one acquisition attempt and a 120-second deadline before the stop and kill deadlines apply. It checks its stop gate again after proxy readiness and reports readiness only after a successful grab and event-loop step. [Build and host checks](build.md#stage-6-updated-kernel-passthrough-executable)

After specific approval for the ELF and temporary ADB execution:

1. Confirm the exact environment from the 11 November record, including driver, manager, ABI, page size and `ksud` digest. Confirm the installed alpha.3 worker digest, saved generation 5, latch set, active generation 0, no pending module operations and no live input worker or proxy. Keep policy and enrollment hashes in the private evidence directory. Stop if any preflight field differs.
2. Create the private, root-owned mode `0700` directory `/data/adb/earkey_control_reforge_check`. Stage and hash the reviewed ELF in it. Run `check` first. Require one target, successful open and revalidation, and neutral input. Run `proxy-check` next, with the physical buttons idle. Require Android's `KEYBOARD | EXTERNAL` classification, the same `Generic.kl` and `Generic.kcm` paths, no configuration override, proxy exclusion and confirmed teardown. Both commands leave the physical input ungrabbed.
3. With the earphones already connected, start music at a comfortable volume. Run `supervise` once and wait for `worker_forwarding=ready` before the maintainer presses the inline controls. Check one short Center, Volume Up and Volume Down press, then check the phone's own volume controls and ongoing audio. Record the expected and observed action for each. Require `worker_blocked_groups=none`, equal nonempty physical and forwarded press and release counts for each tested control, orderly stop and zero supervisor failures. Use the post-stop Center and volume checks to confirm direct physical input returns. D09 still waives repeated reconnect cycles.
4. On request or any unexpected action, signal the verified diagnostic supervisor with `SIGTERM` or create its private mode `0600` `stay-inactive` marker. Its gate closes and the supervisor requests stop, with kill escalation on timeout. The worker also has parent-death protection. The supervisor may remain idle until the session deadline after an early stop. Confirm worker exit, proxy removal and restored physical controls before removing the temporary executable, marker, lock and directory. Recheck the installed module's latch, worker state and before/after policy and enrollment digests. Retain only reviewed summaries in Git.

The first session covers short presses and basic audio and phone controls in the root-shell context. Updated-kernel service-hook, call, microphone, wake, failure and broader preservation checks remain pending. Passing this session allows a revised upgrade plan for review. Suppression requires its own approved artifact and device test.
