# Privacy and diagnostics

Status: requirements for implementation, not claims about a running collector. Normal operation must be offline. No analytics, CDNs, remote fonts, remote scripts, network control server, or automatic diagnostic upload.

Default diagnostics are off. Runtime logs must be bounded and rotated, recording concise state/reason, module version/commit, environment versions, backend, sanitized selected identity, requested/saved/active generation and limited per-group counters. Never log every press, unrelated keyboard input, raw audio, USB serials, contacts, notifications or call contents by default. A control filter is not permission to monitor the user's general input.

Enrollment captures only selected candidates within an explicit finite recording window with bounded size and a visible stop control. Keep raw records privately outside source control; prefer non-sensitive stable identity combinations over serials. Public extracts retain necessary build versions while removing serial/account/contact details and unrelated logs.

Exports require local preview and sanitization before sharing and never upload automatically. Review fixtures individually and label synthetic versus physical origin. Ignore rules reduce accidents but do not replace staged-diff review. Private paths (`private/`, `diagnostics/`, `device-evidence/`) and generated output are excluded from ordinary commits.

Root-owned policy/data is separate from executable code, with validated ownership, file type, paths and private permissions. Explicitly document reset/forget/uninstall retention and remove only module-owned data. Test hostile metadata and private reporting through [security policy](../SECURITY.md); never require unrestricted system logs in a public issue.
