# Stage 6 plan — KernelSU integration

Date: 11 November 2026. Stage 6 turns the Stage 4 and 5 diagnostics into the installable `earkey_control` module. The backlog exit criterion is unchanged: an experimental release candidate with complete reference-phone evidence. It covers boot with and without the accessory, reconnect, live change, update, disable, uninstall and rescue, and mountless operation without a metamodule where supported.

| Increment | Scope | State |
| --- | --- | --- |
| 6.1 Root control commands | `enroll`, `status`, `apply <selection> [enable]`, `enable`, `disable`, `pause`, `resume`, `disable-next-boot`, `enable-next-boot` over the private store. Enrollment only from a unique live observation, policy writes that keep recovery state, and status without serials | implemented, host-tested |
| 6.2 Production worker loop | `earkey-worker run`: load enrollments and the verified policy each pass. Wait without failing while disabled, recovery-inactive, unreadable, accessory-absent, not neutral or grab-conflicted. Forward with the policy's blocked groups, reacquire on a new saved generation, and return to waiting on unplug | implemented, end-to-end container test |
| 6.3 Supervisor daemon | `earkey-worker daemon`: unbounded supervision of `run` with the real persisted restart latch, `clear-latch` to resume, `worker.status`/`supervisor.status` runtime files, and `status` reporting saved and active generations separately | implemented, container-tested |
| 6.4 Module package | `module/` plus `tools/package-module.py`: KernelSU boot-mode, arm64 and API 31 installer checks; a 0700 private store created or kept; `service.sh` starts the daemon only for the pinned digest; `uninstall.sh` stops only this module's processes and removes the store; runtime license files bundled; inactive after a fresh install | implemented, host-tested; not installed |
| 6.5 Offline WebUI | TypeScript/HTML/CSS through the KernelSU WebUI bridge, calling only the fixed control commands. Accessible and offline | planned |
| 6.6 Reference evidence | The Stage 6 physical cases on the reference phone | planned |

Each increment goes through its own PR. Phone runs need separate maintainer authorization.

## Module package 0.1.0-alpha.1

The package is built by `tools/package-module.py` from the pinned worker. Two clean builds from `8957c89` gave identical 923,360-byte ELFs, SHA-256 `3327239e45c7179e302fda9023cd784348277006c9537daec9002631f441d26c`. They pass the NDK pins and ELF checks: API 31, four `LOAD` segments aligned to `0x4000`, only `libc`/`libdl`, all 93 imports in API 31, and no host paths. The ELF is kept at `target/stage6/8957c89/earkey-worker-arm64-api31`. The ZIP has `module.prop` at its root, the scripts, `skip_mount`, `bin/earkey-worker` with its digest, `LICENSE`, `THIRD_PARTY_NOTICES.md`, and `licenses/<crate>-<version>/` for every crate in the Android dependency tree. Two builds of `earkey-control-0.1.0-alpha.1.zip` were byte-identical: 526,427 bytes, SHA-256 `4d515ea2bd4d9b5a7a49a5aa40478ea9408e546bdf222c394ef1869970300955`.

**Install behavior:**
- `customize.sh` requires the boot-mode KernelSU installer (`BOOTMODE=true`, `KSU=true`), arm64 and API 31 or later.
- It creates `/data/adb/earkey_control` as `0:0` `0700`, or keeps an existing one, so updates preserve settings. It refuses a symlinked store.
- With no policy, the policy is inactive and disabled, so the daemon starts and the worker waits as `disabled` without opening any device.

**Service and uninstall:**
- `service.sh` starts `earkey-worker daemon` in the background, only if the binary matches its digest and the store is a real directory.
- `uninstall.sh` runs when the manager prunes the module (before the services stage at boot), or during a full KernelSU uninstall. It stops only processes whose executable is this module's binary (`daemon` with SIGTERM, a 5 s bound, then SIGKILL; then any `run`). It then **removes the private store, including the policy, the enrollment serial and the runtime status**. Keeping settings across an uninstall is not offered yet.

**Use, as root, with `M=/data/adb/modules/earkey_control/bin/earkey-worker`:**
1. `$M enroll` with the earphones connected.
2. `$M apply block-all enable`, or `$M apply center enable`, and so on.
3. `$M status` reports the saved and active generations.

Stopping without the earphone buttons: `$M pause` or `$M disable-next-boot`. After repeated faults: `$M clear-latch`.

**First install plan (needs authorization):**
1. Install the ZIP with `ksud module install`.
2. Reboot and confirm the daemon runs, with `worker.status` showing `disabled`.
3. `enroll`, then `apply block-all enable`. Confirm `active_generation` matches and that the maintainer sees the earphone buttons blocked and the phone keys working.
4. `apply center enable` live; confirm only Center is blocked.
5. `pause`, then `resume`.
6. Unplug, then replug.
7. Reboot with the earphones connected and confirm blocking resumes.
8. Mark the module for removal, reboot, and confirm the store and processes are gone.
