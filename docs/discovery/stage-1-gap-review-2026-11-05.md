# Stage 1 gap review — 5 November 2026

**Stage 1 remains incomplete.** This is a host-side review of the local evidence at `e782887426e24d7f19e31c812904910dcb573318`, not a new phone test. The [checkpoint](reference-2026-11-04.md) supplies measurements and the [contract](../project-contract.md) supplies authority. No additional count waiver, device backend, prototype permission or support claim is introduced here.

Update, 6 November: the checkpoint now includes paused-start Center doubles from source `5a3352ef724bcbe25ee1d9c2fb50a323f43f205b`. One group and a later four-group window have reported track changes; the latter four have explicit one-change-each/paused-continuation confirmation. The earlier rerun-requested window stays separate. This updates the evidence inventory without closing Stage 1.

The later paused-start Center holds use source `a78ad22291838c1a149a8a14cf430acde5d9af44`: five individually reset windows, each with reported assistant activation and a later clarification that music stayed paused in all five after assistant listening ended. Complete frames and endpoint/reader checks passed; these baseline observations add no module qualification.

## Discovery evidence and next checks

| Item | Current evidence | Remaining action |
| --- | --- | --- |
| Transport and control collection | Dedicated USB HID interface, separate audio, detailed key-only capabilities, private unique attributes and stable USB/interface checks | Production matching, duplicate handling and post-open validation are not implemented; retain essential-device denials |
| Reconnect identity | One controlled cycle; later allocation-suffix changes reviewed against stable attributes | D09 waives more physical reference cycles; request none. Software hotplug/revalidation still needs later tests |
| Short Center answer/end | One instrumented call plus maintainer verification | D10 closes this reference repetition item; request no further repeat calls. This does not establish other call gestures |
| Playback and configured off/Dozing baseline | Short controls, Center holds/doubles and volume holds/doubles measured with stated limits | Preserve the reported wake/assistant and pulse behavior; no fully Asleep or continuous-state claim |
| Paused playback | Volume taps/doubles and paired unchanged media indices measured; Center doubles have one-plus-four windows with stated user-report limits; five individual Center holds have reported assistant activation and confirmed paused continuation | Paused volume holds and other relevant unmeasured combinations still need evidence; do not infer them from playing music |
| Incoming/ongoing call contexts | Qualified Center short answer/end and ongoing short volume baseline; interrupted attempts separate | Incoming held/double Center and volume behavior, and ongoing held/double controls remain unmeasured. Any future check needs a consenting/local setup and bounded candidate-only capture; D10 short-call repetition stays closed |
| Unrelated input | Device previously reported unavailable; Not run, not waived or Not applicable | Use an identified available unrelated input device when one is available; preserve audio/earphone connection and independent phone controls |
| Audio/microphone and phone controls | Qualitative listening, USB recording, call audibility, phone power/volume/touch baselines | Exact telephony microphone-path and quantitative comparisons remain unestablished; no module preservation result |
| Additional advertised controls | Only three physical inline buttons reported; other evdev capabilities advertised | Unobserved signals cannot become independent Custom controls or guessed mappings |
| Native candidate decision | Key-only scoping and root/kernel prerequisites support further investigation | Retain conditional native candidate; close remaining discovery evidence before dependent implementation. Active service/proxy equivalence remains a later physical gate |

This list makes existing gaps concrete; it does not demand an unbounded Cartesian product of every possible app/state/gesture. Each remaining context must be measured, explicitly limited or amended by the maintainer before its behavior is claimed. No additional hardware test occurred in this review.

## Later gates kept separate

The specification's discovery instructions distinguish root-shell metadata from later real service-context testing. Actual module-service permissions, uinput creation/readiness/Android mapping, exclusive grab, passthrough parity and bounded recovery belong to the separately authorized Stage 4 prototype after Stages 1–3 and pinned Android build inputs. They have not run and cannot be inferred from the current root predicates. Boot/lifecycle/stress and release-byte qualification retain their later gates. No installation, reboot or interception is authorized by this review.

## Independent work prepared now

The [synthetic protocol summaries](../../tests/fixtures/README.md) prepare host regression inputs for the observed scan-free release/nonzero synchronization edge case and related protocol rules. They contain no hardware profile or Android action result. Their expected summaries can be checked on the host, while the real policy/identity model, Rust engine, fuzzing, Android build and physical qualification remain future work. This preparation changes no Stage 1 exit status and no compatibility level.
