# Enrollment and Stage 1 discovery

Status: read-only phone access was explicitly authorized on 4 November 2026 and Stage 1 is in progress. The [reference checkpoint](discovery/reference-2026-11-04.md) records partial measurements and missing gates; its sanitized publication was explicitly approved. No profile or blocking prototype is installed.

1. Record exact variant, Android/API, full PixelOS build, kernel build/release, executable ABIs, root implementation/fork and kernel/userspace/manager versions, SELinux enforcing state and intended module service context. Do not derive them from the phone name.
2. Inventory candidate input capabilities before insertion, after insertion and after removal. Correlate USB enumeration if present, audio state, sysfs parentage and control collections; Type-C alone does not identify transport. Record any other remapping module.
3. Capture relevant button short/sustained/release/double/manufacturer gestures with filtering absent. Preserve complete frames, timestamps, Linux symbolic/numeric values and numeric base, HID scan data, Android mapping and observed action. Keep namespaces separate.
4. Repeat each mapping at least five times and insertion at least three times. Cover playback, paused, screen locked/off, incoming and ongoing consenting/local test calls; no emergency numbers.
5. Establish baseline audio playback, microphone recording, call route, plug/unplug, physical phone power/volume, touch and an unrelated input device including media controls. Compare baseline battery idle behavior.
6. Identify whether switches, phone controls, keyboards or other essential capabilities share a candidate. Reject mixed nodes. Group only collections independently proven to belong to the accessory. Explain serial-specific versus model-level enrollment and ambiguous duplicates.
7. Determine stable matching attributes across reconnect; collect boot stability when an explicitly authorized boot observation is available. Record gaps. Stage 1 produces a profile candidate, not permission to activate it.
8. Record evdev/uinput presence and intended service-domain access evidence. Do not mistake root-shell permission for service access; service-context probes must be validated before Stage 4. No grab or proxy experiment is part of Stage 1.

Use AOSP [getevent documentation](https://source.android.com/docs/core/interaction/input/getevent) and current input-service diagnostics when preparing authorized commands; syntax must be reviewed for the measured build. Candidate-only recording needs a finite timeout, bounded size and visible stop control. Do not continuously record all keyboard input.

Outputs: sanitized environment report, candidate collection report with essential-input exclusions, button → Linux/HID → Android → action matrix, scoped fixtures labeled physical or synthetic, mapping/permission evidence and explicit primary-backend eligibility decision. Mark unobserved or unsupported gestures; unknowns are never default mappings. Store raw diagnostics privately per [privacy](privacy.md).
