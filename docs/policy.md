# Policy and migration contract

Stage 2 implements the initial schema and host-side storage and identity checks in [earkey-policy](../crates/earkey-policy). ADR-006 selects `/data/adb/earkey_control/policy.json` for all managers. The installer will provision the root-owned directory at mode 0700. The library checks that directory and requires regular, single-link root-owned policy files at mode 0600.

## Schema v1

Every field is required. The top level, profiles, diagnostics and recovery are JSON objects. Lists are ordered JSON arrays. The parser rejects unknown and duplicate members, malformed UTF-8, extra JSON values, unsupported versions and positional arrays in place of objects.

| Field | Allowed values and behavior |
| --- | --- |
| `schema_version` | Integer `1` |
| `generation` | Unsigned 64-bit integer. Zero identifies missing storage or the first write request. Saved generations start at one |
| `enabled` | Boolean master enable. An enabled policy requires an enabled enrolled profile |
| `preset` | `block_all` or `custom` |
| `profiles` | Zero to eight enrolled profiles with distinct serials |
| `requested_backend` | `evdev_uinput`, the candidate backend |
| `unknown_accessory` | `ignore` |
| `diagnostics` | `enabled` boolean, `seconds` and `max_bytes` integers. Disabled requires both limits to be zero. Enabled allows 1–120 seconds and 1–65,536 bytes |
| `recovery` | `paused` and `boot_disabled` booleans, `latch` either `clear` or `restart_limit` |

Each profile requires `evidence`, `usb_serial`, `enabled`, and `blocked_groups`. The evidence IDs `xiaomi_pixelos20261107` (Stage 1 kernel) `xiaomi_pixelos_reforge20261110` (the reforge kernel, from the [10 November revalidation](discovery/reforge-revalidation-2026-11-10.md)) and `xiaomi_pixelos_reforge20261111` (the updated reforge kernel, from the [11 November revalidation](discovery/reforge-revalidation-2026-11-11.md)) resolve to the same measured reference identity, capabilities and mapping constraints in code. Each matches its pinned device model, codename and API level, whatever the running ROM build and kernel ([decision 0005](decisions/0005-rom-and-kernel-independent-evidence.md)). `enroll` and `check` select the newest matching evidence. There is one verified collection in this evidence set. The serial must match an independently verified enrollment supplied by the trusted caller. Serials are nonblank, up to 128 UTF-8 bytes, and contain no control characters. There is no built-in enrollment or model-wide fallback. When one accessory has enrollments under several evidence IDs, `apply` writes a single profile from its latest enrollment, because profiles must have distinct serials. Policy bytes contain private serials and are suitable only for private storage or a trusted root caller.

`blocked_groups` is empty for Block all, which expands to all measured controls in the profile. Custom accepts a unique subset of `center`, `volume_up` and `volume_down`. An empty Custom selection yields no target. The schema accepts at most four nested object or array levels including the top-level object. Input is limited to 16 KiB, including whitespace. Fresh storage has no profiles, generation zero, master enable off and diagnostics off. [Synthetic example](../tests/fixtures/identity/reference-policy.json)

| Group | Linux key | HID page and usage | Android mapping | Effects presented together |
| --- | --- | --- | --- | --- |
| Center | `0x00a4` | `0x000c:0x00cd` | `MEDIA_PLAY_PAUSE` | Playback, track changes, assistant and screen wake, cellular answer/end. Maintainer also reports ongoing-call mute/unmute and incoming-call rejection |
| Volume Up | `0x0073` | `0x000c:0x00e9` | `VOLUME_UP` | Raise volume or show its panel. Maintainer reports silencing an incoming ringtone |
| Volume Down | `0x0072` | `0x000c:0x00ea` | `VOLUME_DOWN` | Lower volume or show its panel. Maintainer reports silencing an incoming ringtone |

The API separates captured mappings and observed actions from the maintainer-reported effects. Additional advertised keys remain outside the three groups. Separate call, assistant, track, wake, mute and ringtone controls require separately identifiable measured signals. [Discovery evidence](discovery/reference-2026-11-04.md)

## Identity and target selection

A candidate must match the independently enrolled serial, measured USB descriptors, physical HID parent and driver, evdev ID/name, complete event/key/scan/property sets, selected layout digest and exact mapped reference environment. The inventory provider must take those observations from the actual device and Android mapping. A missing observation, changed layout, extra event type or key, mixed node, keyboard or virtual device cannot qualify. Duplicate matches reject selection. Allocation numbers never establish hardware identity.

The returned target carries the policy generation and a transient node instance. Post-open validation compares observations from the opened descriptor with a fresh inventory that still resolves the same unique target. A reconnect requires a new selection and post-open validation. The host model consumes snapshots. Android discovery, enrollment, descriptor acquisition, service permission checks and actual interception are later work.

## Transactions and updates

A save takes the generation the caller edited. Updates lock the private directory, validate existing and requested policies, reject stale writers, preserve recovery fields, and assign the next generation. The store writes an exclusive same-directory temporary file, synchronizes its contents, atomically replaces the old file, then synchronizes the directory. If that last sync fails, the API reports uncertain durability and identifies the replacement generation so the caller can reload and reconcile. Ordinary edits cannot clear pause or recovery latches. Dedicated generation-checked operations handle pause, resume, next-boot disable/enable and the restart latch; the supervised lifecycle and root interface own when those operations are invoked. The host supervisor exposes explicit latch clear but does not persist state itself; the adapter must write the durable latch before considering any future start.

Boot disable creates and synchronizes the private `stay-inactive` marker before replacing the policy. Boot enable validates the marker, replaces the policy, then removes and synchronizes the marker. If removal fails after the save, the API reports the saved generation and an incomplete recovery, and the marker still inhibits pre-open access. A failed boot disable leaves the marker in place rather than failing open.

The supervisor never opens input devices, so it has no verified enrollments. `Store::recovery` applies every storage and policy check except matching profiles to enrollments, and returns only the recovery state. `Store::latch_restart_limit` uses the same check to persist the restart latch at the current generation, preserving the rest of the policy; it changes nothing if the latch is already set. Parsing without enrollments is crate-private, so device selection still comes only from `Store::load` with verified enrollments. A gate built from this read can only allow the worker to start; the worker must load the full policy before opening any candidate. The `stay-inactive` marker is still checked separately.

Enrollments live in a separate root-owned `0600` `enrollments.json`, schema version 1, holding at most eight `{evidence, usb_serial}` records with no duplicates. Only `Store::enroll` writes it, with a `VerifiedEnrollment` from `VerifiedEnrollment::from_observation`, which requires a live candidate that matches the reviewed evidence's environment, identity and capabilities. Ordinary policy saves cannot change it. The worker reads `Store::enrollments` to verify the policy before selecting devices. See [decision record 0003](decisions/0003-enrollment-record.md), which is Accepted.

A successful save reports a saved generation only. The worker must independently acknowledge the exact generation before the UI reports active policy. The model refuses new targets when inactive or when recovery inhibits activation. Live application must preserve the current press sequence and honor the separate future-boot and pause controls. Corrupt storage yields an error and requires the consumer to remain inactive. A rejected update leaves the last good policy intact.

## Migration and rollback

V1 is the first durable schema. There is no earlier format to migrate. The store refuses to overwrite an unreadable, corrupt or future-version policy, including with factory defaults. When a successor schema is introduced, the migration must validate v1, durably retain a private compatible backup, and validate the successor before atomic replacement. A downgraded binary stays inactive until an explicit recovery operation restores a reviewed compatible backup. Test the backup, restore and failure behavior when a migration and installer exist. [Validation T23](validation-plan.md#physical-t01t28-matrix)
