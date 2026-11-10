# Reference profile revalidation on the reforge kernel — 10 November 2026

The maintainer installed a new kernel on the reference phone and approved a read-only re-measurement of the reference profile. Nothing was installed, written or rebooted, and no input event was read. The earphones were connected with no button pressed. Measurements were taken over the existing wireless ADB root route, because the earphones occupy the phone's only USB-C port. Raw records, including the serial and Android's device descriptor and unique ID, are kept privately.

## Environment

| Field | Source | Stage 1 evidence | Measured |
| --- | --- | --- | --- |
| Model | `ro.product.model` | `M2101K6P` | same |
| Device | `ro.product.device` | `sweetin` | same |
| API | `ro.build.version.sdk` | `37` | same |
| ROM | `ro.custom.version` | `PixelOS_sweet-17.0-20261030-2116` | same |
| Build ID | `ro.build.id` | `CP2A.260605.016` | same |
| Incremental | `ro.build.version.incremental` | `1793394994` | same |
| Kernel release | `/proc/version` | `4.14.357-openela-VantomKernel` | `4.14.357-reforge-g9b21e3df6c0a` |
| Kernel build | `/proc/version` | `#1 SMP PREEMPT Sun Oct 11 07:45:56 UTC 2026` | `#2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026` |

`uname -r` on the new kernel still reports `4.14.357-openela-VantomKernel`, which is consistent with uname spoofing. An environment reader must use `/proc/version`. `ro.custom.version` is identified here as the ROM property; Stage 1 recorded the value without naming its property.

## Accessory

Exactly one input node was named `Generic Xiaomi Type-C Earphones 1` (`event9` on this boot; event numbers are runtime details). Its sysfs path has the shape the worker inventory requires: `…/usb1/1-1/1-1:1.3/0003:0020:0B21.0001/input/input10/event9`. `/dev/input/event9` is character device 13:73.

| Field | Measured, all equal to Stage 1 |
| --- | --- |
| Input ID | bus `0x0003`, vendor `0x0020`, product `0x0b21`, version `0x0201` |
| Capabilities | `ev=13` (SYN, KEY, MSC), `msc=10` (SCAN), `properties=0` |
| Key bitmap | `40 0 0 0 0 0 0 3800000800 e000000000000 0`, decoding to `0x71 0x72 0x73 0x8b 0xa3 0xa4 0xa5 0x246` |
| HID collection | `0003:0020:0B21.0001` |
| USB interface | `1-1:1.3`, number `03`, class `03`, driver `usbhid` |
| USB device | `0x0020:0x0b21`, bcdDevice `0x0100`, product `Xiaomi Type-C Earphones 1` |
| Serial | 14 characters. Its SHA-256 matched a value present in 119 Stage 1 private records; the value itself is not published |
| Android | `KEYBOARD \| EXTERNAL`, location `usb-xhci-hcd.0.auto-1/input3`, `ConfigurationFile: <none>` |
| Key layout | `/system/usr/keylayout/Generic.kl`, SHA-256 `4d28d904e72ea3beb4d32d73437cf9e7f1ed297a436d51d53b1a248a5cd5f773` |
| Key character map | `/system/usr/keychars/Generic.kcm`, SHA-256 `dede5e113662a2441bfbaa3e679fda2048bb362bcbbf7ce245838183d22fb9b8` |

No `Vendor_0020*`, `Xiaomi` or `Type-C` layout file exists under the ODM, vendor, system or data key layout directories.

## Conclusion and limits

The accessory identity, capability fingerprint and selected layout are unchanged. Only the kernel release and build differ. A new evidence ID can pin the reforge environment with the same identity and capability rules, while the Stage 1 evidence stays valid for its own kernel.

Button event sequences, scan codes, actions and preservation behavior were not re-measured on the new kernel. The Stage 1 control mapping is carried forward on the basis of the unchanged HID collection, capabilities and layout, and Stage 4 passthrough tests must confirm it. This is not module support or physical acceptance.
