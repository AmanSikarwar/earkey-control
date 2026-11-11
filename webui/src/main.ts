import {
  type GroupName,
  type Operation,
  type Status,
  activeGeneration,
  applyCommand,
  command,
  controls,
  describe,
  first,
  label,
  parse,
  result,
  savedGroups,
  supervisor,
} from "./status.js";

declare global {
  interface Window {
    ksu?: { exec(command: string): string };
  }
}

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

let status: Status | null = null;
let dirty = false;
let requestedGeneration: string | undefined;

function exec(line: string): string {
  if (!window.ksu) throw new Error("KernelSU bridge unavailable");
  return window.ksu.exec(`timeout -s KILL 15 ${line} 2>&1`);
}

function announce(text: string, ok: boolean) {
  const message = $("message");
  message.textContent = text;
  message.dataset.ok = String(ok);
}

function run(operation: string, line: string) {
  requestedGeneration = undefined;
  try {
    const outcome = result(operation, exec(line));
    if (outcome.ok) {
      requestedGeneration = outcome.generation;
      announce(requestedGeneration
        ? `Settings ${requestedGeneration} saved. Check the active settings below.`
        : "Earphones enrolled. Choose which controls to block.", true);
      dirty = false;
    } else {
      announce(`Failed: ${outcome.text}`, false);
    }
  } catch (error) {
    announce(`Failed: ${String(error)}`, false);
  }
  refresh();
}

function refresh() {
  try {
    status = window.ksu ? parse(exec(command("status"))) : null;
  } catch {
    status = null;
  }
  render();
}

function selectedGroups(): GroupName[] {
  return Array.from(document.querySelectorAll<HTMLInputElement>("#groups input:checked")).map(
    (input) => input.value as GroupName,
  );
}

function renderMode() {
  const custom = $<HTMLInputElement>("mode-custom").checked;
  $("custom").hidden = !custom;
  $("all-effects").hidden = custom || !$("all-effects").textContent;
}

function renderGroups(s: Status) {
  const list = $("groups");
  const known = controls(s);
  const saved = savedGroups(s);
  if (list.dataset.names === known.map((c) => c.name).join()) {
    if (!dirty) {
      for (const input of list.querySelectorAll<HTMLInputElement>("input")) {
        input.checked = saved.includes(input.value as GroupName);
      }
    }
    return;
  }
  list.replaceChildren();
  list.dataset.names = known.map((c) => c.name).join();
  for (const control of known) {
    const item = document.createElement("li");
    const input = document.createElement("input");
    input.type = "checkbox";
    input.id = `group-${control.name}`;
    input.value = control.name;
    input.checked = saved.includes(control.name);
    input.setAttribute("aria-describedby", `effects-${control.name}`);
    const name = document.createElement("label");
    name.htmlFor = input.id;
    name.textContent = label(control.name);
    const effects = document.createElement("p");
    effects.id = `effects-${control.name}`;
    effects.className = "effects";
    effects.textContent = `Also blocks: ${control.effects.join("; ")}.`;
    item.append(input, name, effects);
    list.append(item);
  }
}

function render() {
  const d = describe(status);
  if ($("state").textContent !== d.state) $("state").textContent = d.state;
  $("state").dataset.state = d.state;
  if ($("reason").textContent !== d.reason) $("reason").textContent = d.reason;
  const action = $<HTMLButtonElement>("action");
  action.hidden = !d.action;
  action.textContent = d.actionLabel ?? "";
  action.onclick = d.action ? () => run(d.action as Operation, command(d.action as Operation)) : null;

  const s = status;
  const usable = !!s && first(s, "enrollments") !== "0" && controls(s).length > 0;
  $("settings").hidden = !usable;
  $("recovery").hidden = !s;
  const pause = $<HTMLButtonElement>("pause");
  pause.disabled = !s;
  if (!s) return;

  const paused = first(s, "paused") === "true";
  const releasing = d.state === "Discovering";
  pause.disabled = paused && releasing;
  pause.textContent = paused ? (releasing ? "Pausing…" : "Resume") : "Pause now";

  const bootOff = first(s, "boot_disabled") === "true" || first(s, "stay_inactive") === "true";
  const boot = $<HTMLButtonElement>("boot");
  boot.textContent = bootOff ? "Allow blocking again" : "Keep off, including after reboot";
  boot.disabled = bootOff && releasing;
  $<HTMLButtonElement>("latch").hidden = first(s, "restart_latch") !== "true";

  $("profile").textContent = first(s, "enrolled_evidence") ?? "none";
  $("requested-generation").textContent = requestedGeneration ?? "none";
  $("saved-generation").textContent = first(s, "saved_generation") ?? "none";
  const active = activeGeneration(s);
  $("active-generation").textContent = active === "0" ? "none reported" : active;
  $("failures").textContent = supervisor(s, "failures") ?? "unknown";
  $("last-fault").textContent = supervisor(s, "last_fault") ?? "unknown";

  if (!usable) return;
  renderGroups(s);
  const center = controls(s).find((c) => c.name === "center");
  $("all-effects").textContent = center ? `Blocking Center also blocks ${center.effects.join("; ")}.` : "";
  if (!dirty) {
    $<HTMLInputElement>("enabled").checked = first(s, "enabled") === "true";
    const all = controls(s).every((c) => savedGroups(s).includes(c.name));
    $<HTMLInputElement>(all ? "mode-all" : "mode-custom").checked = true;
  }
  renderMode();
}

function init() {
  $("pause").addEventListener("click", () => {
    const op = first(status ?? new Map(), "paused") === "true" ? "resume" : "pause";
    run(op, command(op));
  });
  $("boot").addEventListener("click", () => {
    const s = status ?? new Map();
    const op = first(s, "boot_disabled") === "true" || first(s, "stay_inactive") === "true"
      ? "enable-next-boot"
      : "disable-next-boot";
    run(op, command(op));
  });
  $("latch").addEventListener("click", () => run("clear-latch", command("clear-latch")));
  $("settings").addEventListener("change", () => {
    dirty = true;
    renderMode();
  });
  $("settings").addEventListener("submit", (event) => {
    event.preventDefault();
    const groups = $<HTMLInputElement>("mode-all").checked ? "block-all" : selectedGroups();
    run("apply", applyCommand(groups, $<HTMLInputElement>("enabled").checked));
  });
  $("refresh").addEventListener("click", refresh);
  refresh();
  setInterval(() => {
    if (document.visibilityState === "visible") refresh();
  }, 3000);
}

init();
