# Contributing

EarKey Control uses MIT in the public repository `AmanSikarwar/earkey-control`. Begin with [AGENTS.md](AGENTS.md), the [contract](docs/project-contract.md), and the [backlog](docs/backlog.md).

## Repository conventions

Use intended base `main` and short branches such as `docs/stage-0-contract`, `feat/policy`, or `fix/held-key`. The base and remote must be verified when an upstream is selected; do not invent a base commit for the empty bootstrap. Give each issue one behavior, preservation constraints, acceptance cases, and a stage gate. Local backlog IDs are planning references, not invented GitHub issue numbers.

Stage only named paths. Before committing, inspect status, staged diff, staged stat, and staged path list. Use the configured real author; stop if none is available. Describe the resulting behavior in focused commits. No generated output, secrets, raw device logs, phone images, or incidental caches. Never reset unrelated work, force-push shared history, or move a published tag.

Synchronize with the selected upstream base before a PR, resolve conflicts on the branch, and rerun affected checks. Use the [PR template](.github/pull_request_template.md). Keep source review, CI, and physical evidence distinct. Disclose absent independent review for sensitive changes. No self-approval requirement for a solo maintainer.

The default branch `main` is covered by the active [Protection rules ruleset](https://github.com/AmanSikarwar/earkey-control/rules/26527076), verified on 4 November 2026. It requires PRs, prevents branch deletion and force pushes, has no bypass actors, and requires zero approving reviews for the solo maintainer. Private vulnerability reporting is enabled. [PR #2](https://github.com/AmanSikarwar/earkey-control/pull/2) is merged and [issue #1](https://github.com/AmanSikarwar/earkey-control/issues/1) is closed.

No repository CI workflow or required-status-check rule is implemented yet; add stable required check names when their jobs exist. The ruleset permits merge commits, squash and rebase; no single merge method is enforced. Squash remains the proposed default for small issue branches; confirm policy before merging. Delete branches only after a verified merge. The classic branch-protection endpoint reports no classic rule; the separate active ruleset supplies the verified protection.

## Planned code locations

Create these only when their stage needs them; there are no empty implementation scaffolds in Stage 0.

| Location | Responsibility |
| --- | --- |
| `crates/` | Rust policy, identity, event-state processing, worker, supervisor, and root command interface; keep unsafe wrappers small |
| `module/` | Metadata, BusyBox ash lifecycle inputs, safe defaults, and skip_mount; no built binaries in source |
| `webui/` | Offline TypeScript, HTML, CSS, capability-driven screens and focused reusable components |
| `tests/` | Host tests and reviewed synthetic/sanitized fixtures with evidence classification |
| `tools/` | Reproducible build and packaging tools introduced with meaningful checks |
| `docs/` | Requirements, decisions, device-test procedures, and sanitized support records |
| `.github/` | Issue/PR templates and later pinned, least-privilege workflows |

Build and test conventions are in [build](docs/build.md) and [validation](docs/validation-plan.md). Do not treat checks listed there as already implemented. Update compatibility, recovery, privacy, and policy docs when their behavior changes.
