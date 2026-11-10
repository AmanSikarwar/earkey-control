# Stage 6 plan — KernelSU integration

Date: 11 November 2026. Stage 6 turns the Stage 4 and 5 diagnostics into the installable `earkey_control` module. The backlog exit criterion is unchanged: an experimental release candidate with complete reference-phone evidence. It covers boot with and without the accessory, reconnect, live change, update, disable, uninstall and rescue, and mountless operation without a metamodule where supported.

| Increment | Scope | State |
| --- | --- | --- |
| 6.1 Root control commands | `enroll`, `status`, `apply <selection> [enable]`, `enable`, `disable`, `pause`, `resume`, `disable-next-boot`, `enable-next-boot` over the private store. Enrollment only from a unique live observation, policy writes that keep recovery state, and status without serials | implemented, host-tested |
| 6.2 Production worker loop | `earkey-worker run`: load enrollments and the verified policy each pass. Wait without failing while disabled, recovery-inactive, unreadable, accessory-absent, not neutral or grab-conflicted. Forward with the policy's blocked groups, reacquire on a new saved generation, and return to waiting on unplug | implemented, end-to-end container test |
| 6.3 Supervisor daemon | `earkey-worker daemon`: unbounded supervision of `run` with the real persisted restart latch, `clear-latch` to resume, `worker.status`/`supervisor.status` runtime files, and `status` reporting saved and active generations separately | implemented, container-tested |
| 6.4 Module package | `module/` plus `tools/package-module.py`: KernelSU boot-mode, arm64 and API 31 installer checks; a 0700 private store created or kept; `service.sh` starts the daemon only for the pinned digest; `uninstall.sh` stops only this module's processes and removes the store; runtime license files bundled; inactive after a fresh install | 0.1.0-alpha.1 install plan passed on the reference phone; 0.1.0-alpha.2 (readiness-progress fix) passed its boot check on the reference phone |
| 6.5 Offline WebUI | `webui/` (TypeScript 6.0.3): one state with a reason and recovery action, Pause now, master switch, Block all or Custom over the measured controls with their disclosed effects, next-boot disable and latch clear. Only allowlisted worker commands go through the synchronous `ksu.exec` bridge, under a self-only CSP | implemented, host-tested; not run on the phone |
| 6.6 Reference evidence | The Stage 6 physical cases on the reference phone | planned |

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

The `ProgressStalled` failure seen with alpha.1 did not recur. This is one boot, not a measured rate. The module is still installed with Center blocked.
