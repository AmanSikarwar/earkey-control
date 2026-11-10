# Stage 6 plan — KernelSU integration

Date: 11 November 2026. Stage 6 turns the Stage 4 and 5 diagnostics into the installable `earkey_control` module. The backlog exit criterion is unchanged: an experimental release candidate with complete reference-phone evidence. It covers boot with and without the accessory, reconnect, live change, update, disable, uninstall and rescue, and mountless operation without a metamodule where supported.

| Increment | Scope | State |
| --- | --- | --- |
| 6.1 Root control commands | `enroll`, `status`, `apply <selection> [enable]`, `enable`, `disable`, `pause`, `resume`, `disable-next-boot`, `enable-next-boot` over the private store. Enrollment only from a unique live observation, policy writes that keep recovery state, and status without serials | implemented, host-tested |
| 6.2 Production worker loop | Load enrollments and the verified policy. Wait for the accessory without counting failures, and return to waiting on unplug. Forward with the policy's blocked groups, and pick up a new policy generation at a neutral boundary | planned |
| 6.3 Supervisor daemon | Unbounded supervision from the service stage, with the real restart latch, and status reporting requested, saved and active generations separately | planned |
| 6.4 Module package | `module.prop` (`earkey_control`), `customize.sh` prerequisite checks, `service.sh`, `uninstall.sh`, `skip_mount`, the pinned binary, and safe inactive defaults on a fresh install | planned |
| 6.5 Offline WebUI | TypeScript/HTML/CSS through the KernelSU WebUI bridge, calling only the fixed control commands. Accessible and offline | planned |
| 6.6 Reference evidence | The Stage 6 physical cases on the reference phone | planned |

Each increment goes through its own PR. Phone runs need separate maintainer authorization.
