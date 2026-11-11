# Root-manager adapter plan

The KernelSU alpha implements installation, service supervision and root commands. The tested ReSukiSU combination is Experimental and the alpha.4 WebUI awaits device validation. Upstream KernelSU, Magisk and APatch remain separate design targets. Share one policy and core, with one ZIP unless a measured manager requirement calls for separate assets from the same commit. [Stage 6 evidence](stage-6-plan.md), [compatibility](compatibility.md)

| Priority | Planned integration | Required manager-specific evidence |
| --- | --- | --- |
| 1 KernelSU | Ordinary non-blocking service hook, BusyBox ash, offline `webroot/index.html` interface and root commands | Exact implementation/fork, environment markers, userspace/kernel/manager versions, service domain, no-metamodule baseline, WebUI/bridge permissions, install/boot/update/disable/uninstall/rescue |
| 2 Magisk | Minimal lifecycle adapter and manager-independent root commands; optional useful Action entry | Official markers plus probes, supported customization/cleanup conventions, service permissions and real physical acceptance; no assumed built-in WebUI |
| 3 APatch | Minimal APM adapter and root commands; WebUI only if actual host is tested | `APATCH` and version evidence from supported environment, path/service/permission differences, exact metamodule behavior if a future overlay is selected |

KernelSU's compatibility variables can resemble Magisk. A Magisk-like directory or version variable is not sufficient manager identity. Resolve official markers and actual capabilities; conflicting active-root signals abort installation with a reason. Other `su` implementations do not prove a module boot lifecycle.

Installation must check API ≥31, packaged/measured ABI, executable loading, manager support and access prerequisites. Distinguish installable-inactive from supported-active. Keep `module.prop` at ZIP root and use documented `customize.sh`, `service.sh`, `uninstall.sh`, optional `action.sh`, and `skip_mount` as appropriate. No installation in custom recovery. Preserve the manager's WebUI permissions and contexts; do not chmod broadly.

Run long-lived work only from a non-blocking service stage, resolving the hook's own location. No accessory wait/event loop in post-fs-data, preinit/init injection or global service directories. Handle single-instance locking, paused updates and cleanup of verified module-owned processes only. Late loading and instant manager disable are unsupported until tested; document reboot requirements honestly.

Primary sources: [KernelSU module guide](https://kernelsu.org/guide/module.html), [KernelSU WebUI](https://kernelsu.org/guide/module-webui.html), [Magisk development guide](https://topjohnwu.github.io/Magisk/guides.html), [APatch APM guide](https://apatch.dev/apm-guide.html). Research date and APatch documentation ambiguity are in [research](research.md). Do not install or replace a metamodule or infer a common mounting policy from these overlapping layouts.
