import assert from "node:assert/strict";
import test from "node:test";
import { activeGeneration, applyCommand, command, controls, describe, parse, result, savedGroups } from "../dist/status.js";

const W = "/data/adb/modules/earkey_control/bin/earkey-worker";
const base = `enrollments=1
enrolled_evidence=xiaomi_pixelos_reforge20261110
control=center:Play or pause|Change track
control=volume-up:Raise volume
control=volume-down:Lower volume
stay_inactive=false
saved_generation=4
enabled=true
paused=false
boot_disabled=false
restart_latch=false
profiles=1
profile_blocked_groups=Center
worker=state=active:generation=4:blocked=Center
worker=active_generation=4
supervisor=phase=Running
supervisor=failures=0
supervisor=last_fault=none
`;
const edit = (text, key, value) => text.replace(new RegExp(`^${key}=.*$`, "m"), `${key}=${value}`);

test("status states, reasons and recovery actions", () => {
  assert.deepEqual(describe(null).state, "Unsupported");
  assert.deepEqual(describe(parse(base)), { state: "Active", reason: "Blocking Center." });
  assert.equal(describe(parse("enrollments=0\nenrolled_evidence=none\n")).action, "enroll");
  assert.equal(describe(parse("enrollments=1\npolicy=error:Io\n")).state, "Fault");
  assert.equal(describe(parse(edit(base, "restart_latch", "true"))).action, "clear-latch");
  const stopped = edit(base, "supervisor", "phase=Inactive(Paused)");
  assert.equal(describe(parse(edit(stopped, "stay_inactive", "true"))).action, "enable-next-boot");
  assert.equal(describe(parse(edit(stopped, "paused", "true"))).action, "resume");
  const disabled = edit(edit(base, "enabled", "false"), "worker", "state=disabled");
  assert.equal(describe(parse(disabled)).action, "enable");
  assert.equal(describe(parse(edit(base, "worker", "state=waiting_accessory"))).state, "Waiting for earphones");
  assert.equal(describe(parse(edit(base, "worker", "state=disabled"))).state, "Discovering");
  assert.equal(describe(parse(edit(base, "worker", "state=active:generation=4:blocked=none"))).state, "Ready");
  assert.equal(describe(parse(edit(base, "worker", "state=fault:Stalled"))).reason, "Worker reported: fault:Stalled");
  assert.equal(describe(parse(base.replace(/^supervisor=.*\n/gm, ""))).state, "Fault");
});

test("saved generation and confirmed active generation are independent", () => {
  const pending = parse(edit(base, "saved_generation", "5"));
  assert.equal(activeGeneration(pending), "4");
  assert.deepEqual(describe(pending), { state: "Active", reason: "Blocking Center. Saved settings 5 are pending." });
  const inconsistent = parse(base.replace("worker=active_generation=4", "worker=active_generation=5"));
  assert.equal(activeGeneration(inconsistent), "0");
  assert.equal(describe(parse(edit(base, "supervisor", "phase=Starting"))).state, "Discovering");
  const pause = edit(base, "paused", "true");
  assert.equal(describe(parse(pause)).state, "Discovering");
  assert.equal(describe(parse(edit(pause, "supervisor", "phase=Stopping { cause: Paused, forced: false }"))).state, "Discovering");
  const stopped = parse(edit(pause, "supervisor", "phase=Inactive(Paused)"));
  assert.equal(describe(stopped).state, "Inactive");
  assert.equal(activeGeneration(stopped), "0");
  assert.equal(describe(parse(edit(pause, "supervisor", "phase=Unsafe(StopUnconfirmed)"))).state, "Fault");
});

test("measured controls and saved groups", () => {
  const s = parse(base);
  assert.deepEqual(controls(s).map((c) => c.name), ["center", "volume-up", "volume-down"]);
  assert.deepEqual(controls(s)[0].effects, ["Play or pause", "Change track"]);
  assert.deepEqual(controls(parse("control=mic:x\ncontrol=center\n")), []);
  assert.deepEqual(savedGroups(s), ["center"]);
  assert.deepEqual(savedGroups(parse("profile_blocked_groups=Center,VolumeUp,VolumeDown")), [
    "center", "volume-up", "volume-down",
  ]);
});

test("commands are allowlisted", () => {
  assert.equal(command("pause"), `${W} pause`);
  assert.throws(() => command("status; reboot"));
  assert.equal(applyCommand("block-all", true), `${W} apply block-all enable`);
  assert.equal(applyCommand(["center", "volume-down"], false), `${W} apply center,volume-down`);
  assert.equal(applyCommand([], true), `${W} apply passthrough enable`);
  assert.throws(() => applyCommand(["center;reboot"], true));
  assert.throws(() => applyCommand(["center", "center"], true));
});

test("operation results", () => {
  assert.deepEqual(result("apply", "apply=ok:saved_generation:5\n"), { ok: true, text: "ok:saved_generation:5", generation: "5" });
  assert.equal(result("apply", "apply=ok:saved_generation:18446744073709551615\n").generation, "18446744073709551615");
  assert.equal(result("apply", "apply=ok:active\n").ok, false);
  assert.equal(result("pause", "pause=error:Store(Stale)\n").ok, false);
  assert.equal(result("enroll", "enroll=ok:enrolled:xiaomi_pixelos_reforge20261110\n").ok, true);
  assert.deepEqual(result("enroll", "sh: not found\n"), { ok: false, text: "sh: not found" });
});
