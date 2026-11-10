# Enrollment record decision

Date: 10 November 2026. Status: **Accepted.** The maintainer asked for the enrollment source to be worked out and accepted this record on 10 November 2026. Acceptance does not authorize device access or enrollment on the phone.

## Context

`Policy::parse` accepts a profile only when a `VerifiedEnrollment` with the same evidence and USB serial is supplied, and policy JSON cannot create one. The only constructor was `VerifiedEnrollment::from_reviewed_measurement`, which relies on its caller for provenance. At boot the worker therefore had no trusted source for the serial, and the supervisor deliberately has no enrollments at all. The specification requires stable measured identities, enrollment of unknown accessories only by explicit choice, and that imported profiles cannot bypass protections.

## Decision

- **Provenance:** `VerifiedEnrollment::from_observation(evidence, environment, candidate)` enrolls a serial only when a live inventory candidate matches the reviewed evidence's environment, identity and capabilities. The serial comes from that observation, not from user text or policy JSON.
- **Durable record:** `Store::enroll` writes a separate root-owned `0600` `enrollments.json` in the private store, using the same locked atomic write as the policy. The file has schema version 1, holds at most eight records, rejects duplicates, unknown members, duplicate keys and invalid serials, and refuses to overwrite a malformed file. Re-enrolling an existing record changes nothing.
- **Separation:** ordinary policy saves cannot write enrollments, and `Store::enroll` cannot change policy. A profile still needs both a matching enrollment and a valid policy.
- **Use:** the worker reads `Store::enrollments`, converts the records with `EnrollmentRecord::verified`, then calls `Store::load`. Device selection still requires `Policy::targets` against a fresh inventory and post-open revalidation. The supervisor gate keeps using the recovery-only read and never needs enrollments.

## Alternatives that matter

- **Serial inside policy profiles only:** this is today's schema. Without a separate record, any policy writer could name a serial. It is rejected as the sole source.
- **Live enrollment at every boot:** this would make activation depend on a fresh unprompted match, which the specification forbids for unknown accessories. It is rejected.
- **Model-level enrollment without a serial:** this is ambiguous with duplicate accessories, and the reference evidence is serial-specific. It is deferred.

## Limits and revisit conditions

The enrollment command, the UI flow and the operation to forget an accessory are not implemented. Forgetting will need to remove the enrollment and its profile together. The current reference evidence pins the Stage 1 kernel, so `from_observation` refuses it on the reforge kernel until new evidence is reviewed. Revisit this record if serial-less or multi-collection accessories are added, or if the root command interface changes the trust boundary.
