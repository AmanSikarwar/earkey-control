# Changelog

## Unreleased

- Prepared the Stage 0 project contract, safety architecture, repository conventions, recovery outline, and validation plan against specification v1.1.
- Recorded maintainer decisions: MIT license, public repository under AmanSikarwar, and releases only after implementation and acceptance gates are complete.
- Runtime implementation and all physical compatibility tests remain unstarted.
- Verified the supplied public repository and published baseline; prepared an issue-linked setup review, documented denied administration operations, and routed security inquiries to the reporting policy while private reporting is disabled.
- Verified PR #2 merged and issue #1 closed, private reporting enabled, and an active default-branch ruleset requiring PRs and preventing deletion/force pushes without self-approval or bypasses. Stage 0 exit is verified; Stage 1 phone access remains unauthorized.
- After PR #3 merged, began explicitly authorized read-only Stage 1 discovery. Prepared an environment/candidate report and sanitized physical event fixture, retaining an empty capture and the Center count discrepancy. The maintainer explicitly approved publication of this sanitized checkpoint; Stage 1 remains incomplete.
- Recorded the maintainer's waiver of further reference reconnect-cycle repetitions and confirmation of seven intentional Center presses. Updated the fixture observation without changing captured events; the Center count question is resolved and the waived cycle counts are not reported as passed.
- Verified read-only root access after the maintainer enabled it: UID 0/ksu context, userspace code 35257, kernel code 35240/UAPI 6 built-in, evdev/uinput kernel flags and separate USB audio/HID interfaces. Actual module-service and forwarding behavior remain untested.
