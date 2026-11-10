# Proxy identity and readiness decision

Date: 10 November 2026. Status: **Accepted** by the maintainer on 10 November 2026. This narrows [ADR-002](0001-initial-design.md#adr-002-proxy-identity-and-mapping--validation-strategy-accepted-mapping-pending) for the reference profile. It does not authorize device access, a grab or passthrough.

## Context

ADR-002 requires a distinctly marked virtual device, exclusion from discovery, only verified capabilities, observed Android readiness and one repeat owner, and allows a narrowly documented copy of identity fields. On the reference phone, Android classifies the physical earphones as `KEYBOARD | EXTERNAL` and selects `Generic.kl`, because no `Vendor_0020_Product_0b21*` or name-based layout or configuration exists. AOSP layout lookup tries vendor/product/version, then vendor/product, then the device name, then `Generic.kl`. AOSP's EventHub classifies USB and Bluetooth buses as external unless an input configuration says otherwise. The physical node advertises `EV_SYN`, `EV_KEY` and `EV_MSC`, with no `EV_REP`.

## Decision

- **Identity:** the proxy copies only bus, vendor, product and version from the opened physical descriptor, so Android keeps the `EXTERNAL` classification and the same layout lookup outcome. Its name is the fixed `EarKey Control proxy`, so the name-based lookup cannot find a layout file meant for the accessory. uinput places it under `/sys/devices/virtual`, so the worker inventory marks it virtual with no USB ancestry, and the evidence matcher, which requires a non-virtual USB HID node, can never select it.
- **Capabilities:** the proxy advertises `EV_KEY` with only the evidence's measured control keys (`0x72`, `0x73`, `0xa4`), plus `EV_MSC` with `MSC_SCAN`. The other advertised physical keys are never forwarded and are not advertised.
- **Repeat owner:** the proxy does not set `EV_REP`, so the kernel generates no repeats. Source repeats from the physical node are forwarded once. This matches the physical node, which has no `EV_REP`, and keeps the source as the single repeat owner.
- **Readiness:** after `UI_DEV_CREATE`, the worker resolves the proxy's event node through `UI_GET_SYSNAME` and sysfs. It then waits, within a bounded deadline, until `/dev/input/eventN` exists with the sysfs device number and `system_server` holds an open descriptor to it. That descriptor shows Android's input reader has opened the node. A fixed sleep is not readiness.
- **Teardown:** `UI_DEV_DESTROY` and close on every exit path. The worker confirms that the proxy's event node is gone.

## Alternatives that matter

- **A fully distinct identity (virtual bus, zero IDs):** Android would classify the device as internal, which can change wake and policy behavior relative to the measured baseline. This is rejected for the reference profile, and the decision should be revisited if a distinct identity can be shown to be behaviorally equivalent.
- **A cloned name or serial:** this risks layout and descriptor conflation and makes the proxy harder to distinguish in diagnostics. It is rejected.
- **A layout overlay for the proxy:** this changes the backend classification and needs its own accepted decision.

## Validation required before use

On the reference phone: proxy creation from the services hook, the readiness latency, Android classification `KEYBOARD | EXTERNAL`, `Generic.kl` selected for the proxy, no layout or configuration file for its identity, the inventory and matcher excluding it, and teardown removing the node. Key mapping and action equivalence need forwarded events, which is the Stage 4 passthrough gate.
