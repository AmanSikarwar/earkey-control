# Policy and migration contract

Status: pre-implementation design for Stage 2. No schema or CLI syntax is implemented. ADR-006 selects structured JSON at `/data/adb/earkey_control/policy.json`; `/data/adb/earkey_control/` will be root-owned mode 0700 and durable policy/state files mode 0600. These are engineering choices, not paths supplied by a manager API. Generated runtime state is separate from durable policy, pause and recovery state.

## Required data model

Include an explicit schema version, master-enabled state (false on fresh install), Block all or Custom preset, enrolled profiles, stable identity scope and measured capabilities, mapping-evidence version/environment constraints, enabled profile state, explicit blocked independent groups, requested backend, unknown-accessory ignore behavior, diagnostics limits (off by default), and recovery-latch reason. Report effective backend and active generation as observed runtime state, not a success inferred from saved policy.

The first schema will require all declared policy fields and reject unknown fields rather than silently accepting misspellings. Reject duplicate JSON members, unsupported future versions, conflicting presets, duplicate identities, invalid or wrong-namespace codes, unknown groups, ambiguous matches, malformed text, oversized files and excessive list/string/depth counts. Missing policy means inactive; corrupt stored policy means inactive with a reason. No profile may bypass essential-device denials. Exact bounded limits and formal schema are Stage 2 work and must precede parser use.

Keep Linux codes, HID usages (page and usage), and Android keycodes distinct. Block all expands only to verified controls across the accessory's verified collections. Custom uses supported independent groups; it cannot request an action split absent from measurements. Unsupported requests retain the previous valid policy and explain collateral effects.

Include the reported in-call mute toggle and incoming-call rejection in the potential effects shown before blocking the reference accessory’s Center control. An independent mute option requires evidence of a separately identifiable signal. The call-hold mapping and microphone behavior still need physical verification. [D13](project-contract.md#reference-ongoing-call-center-hold-amendment--7-november-2026), [D15](project-contract.md#reference-incoming-call-center-hold-amendment--7-november-2026)

Before blocking either inline volume control on the reference accessory, disclose that the block may remove the reported ability to silence the ringtone. Verify the gesture mapping and call behavior before offering any separate ringtone action option. [D17](project-contract.md#reference-incoming-call-volume-hold-amendment--7-november-2026)

## Transaction and update behavior

Validate a candidate completely before changing behavior; show capability/collateral effects. Serialize concurrent updates and assign an immutable generation. Use private same-directory temporary storage, validated file types/ownership, no symlink following, atomic replacement and durability operations appropriate to Android. Handle full storage, interruption and rollback without truncating the last good file. A failed save must not report success.

Report requested, saved and active generations separately. Await worker acknowledgment for the exact generation before reporting application. A save can succeed while activation is pending or fails. Keep the prior good active policy on validation failure; invalid runtime identity or a fault releases interception. Apply decisions only to a new press sequence or a neutral whole-device transition.

Provide distinct pause-now, resume-validated-policy, disable-future-boot, safe-inactive-reset and forget-profile operations. Pause persists through updates. Reset/forget must release affected resources safely and disclose exactly what data is removed; neither may silently reactivate the feature.

Migrations require a private validated backup, explicit version transition, failure-safe replacement and a documented downgrade path. An older binary that cannot read a future schema must stay inactive without overwriting it; restore a compatible reviewed backup only through explicit recovery. Installation defaults never overwrite valid policy or pause state. Test T01–T05, T15, T21 and T23.
