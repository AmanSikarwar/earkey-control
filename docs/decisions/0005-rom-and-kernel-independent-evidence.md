# ROM- and kernel-independent evidence decision

Date: 13 November 2026. Status: **Accepted.** The maintainer decided on 13 November 2026 that the worker should not depend on the kernel or the ROM build, ahead of planned kernel and ROM updates. Acceptance does not authorize device access, installation or a reboot.

## Context

Each evidence ID pinned the exact ROM version, build ID, incremental, kernel release and kernel build string. Every kernel or ROM change on the reference phone therefore made the worker report `unsupported_environment` and leave the earphones untouched until a new release added a profile. Two kernel changes did this: reforge `g9b21e3df6c0a` replaced the Stage 1 kernel, and `gc216b1fe2466` replaced it. Each time the accessory identity, capabilities and layout revalidated unchanged, and passthrough and blocking then worked.

## Decision

- `Evidence::matches_environment` compares only the device model, codename and Android API level. The ROM version, build ID, incremental, kernel release and kernel build are still read and validated, so a missing or malformed value still matches nothing, but they are not compared.
- `enroll` and `check` select the newest evidence whose environment matches.
- Every live check is unchanged: the USB identity and driver, the input ID and name, the exact capability bitmaps, the `Generic.kl` digest, no layout or IDC override, revalidation after opening, the neutral start and proxy readiness.

## Consequences and residual risk

- A kernel or ROM update no longer disables blocking by itself. Existing enrollments keep working if the live checks pass.
- A ROM update that changes `Generic.kl` makes the earphones fail the layout check, so the worker leaves them untouched and `enroll` reports `layout`. That needs a new measurement, because the disclosed button effects depend on the mapping.
- An Android version change (API level) still makes the worker report `unsupported_environment`.
- A kernel or ROM can change input behavior that the capability bitmaps and layout digest do not show, such as key repeat timing, how Android dispatches media keys, or the USB HID driver's handling of resets. Those changes are not measured before use. The maintainer accepts this risk for the reference phone. The pause and boot-disable controls remain the rescue path.
- Support claims stay per measured combination. A ROM or kernel is not Verified or Experimental because the worker accepts it; [compatibility](../compatibility.md) still records each combination actually tested.
