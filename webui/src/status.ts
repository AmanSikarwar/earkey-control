export const WORKER = "/data/adb/modules/earkey_control/bin/earkey-worker";

export const OPERATIONS = [
  "status",
  "enroll",
  "enable",
  "disable",
  "pause",
  "resume",
  "disable-next-boot",
  "enable-next-boot",
  "clear-latch",
] as const;
export type Operation = (typeof OPERATIONS)[number];

export const GROUPS = ["center", "volume-up", "volume-down"] as const;
export type GroupName = (typeof GROUPS)[number];

export type Status = Map<string, string[]>;

export type State =
  | "Inactive"
  | "Waiting for earphones"
  | "Discovering"
  | "Ready"
  | "Active"
  | "Unsupported"
  | "Fault";

export interface Description {
  state: State;
  reason: string;
  action?: Operation;
  actionLabel?: string;
}

export interface Control {
  name: GroupName;
  effects: string[];
}

/** Parses `key=value` lines; repeated keys keep every value in order. */
export function parse(text: string): Status {
  const status: Status = new Map();
  for (const line of text.split("\n")) {
    const at = line.indexOf("=");
    if (at <= 0) continue;
    const key = line.slice(0, at);
    const values = status.get(key) ?? [];
    values.push(line.slice(at + 1));
    status.set(key, values);
  }
  return status;
}

export function first(status: Status, key: string): string | undefined {
  return status.get(key)?.[0];
}

function runtime(status: Status, key: string, field: string): string | undefined {
  const prefix = `${field}=`;
  return status
    .get(key)
    ?.find((line) => line.startsWith(prefix))
    ?.slice(prefix.length);
}

export function workerState(status: Status): string | undefined {
  return runtime(status, "worker", "state");
}

export function activeGeneration(status: Status): string {
  return activeReport(status)?.[1] ?? "0";
}

export function supervisor(status: Status, field: string): string | undefined {
  return runtime(status, "supervisor", field);
}

function activeReport(status: Status): RegExpMatchArray | null {
  const phase = supervisor(status, "phase");
  if (phase !== "Running" && !phase?.startsWith("Stopping ")) return null;
  const active = workerState(status)?.match(/^active:generation=([1-9]\d*):blocked=(.+)$/);
  return active && active[1] === runtime(status, "worker", "active_generation") ? active : null;
}

export function controls(status: Status): Control[] {
  const found: Control[] = [];
  for (const line of status.get("control") ?? []) {
    const at = line.indexOf(":");
    const name = line.slice(0, at) as GroupName;
    if (at > 0 && GROUPS.includes(name) && !found.some((c) => c.name === name)) {
      found.push({ name, effects: line.slice(at + 1).split("|").filter(Boolean) });
    }
  }
  return found;
}

const GROUP_FROM_STATUS: Record<string, GroupName> = {
  Center: "center",
  VolumeUp: "volume-up",
  VolumeDown: "volume-down",
};

/** The saved blocked groups of the first profile. */
export function savedGroups(status: Status): GroupName[] {
  const value = first(status, "profile_blocked_groups");
  if (!value || value === "none") return [];
  return value.split(",").flatMap((g) => (GROUP_FROM_STATUS[g] ? [GROUP_FROM_STATUS[g]] : []));
}

export function label(name: GroupName): string {
  return { center: "Center", "volume-up": "Volume Up", "volume-down": "Volume Down" }[name];
}

/** One user-facing state with a reason and, when one exists, the recovery operation. */
export function describe(status: Status | null): Description {
  if (!status) {
    return { state: "Unsupported", reason: "Open this page from the KernelSU manager." };
  }
  const failed = (key: string) => first(status, key)?.startsWith("error:");
  if (!status.has("enrollments") || failed("enrollments") || failed("policy")) {
    return { state: "Fault", reason: "The saved settings could not be read." };
  }
  const phase = supervisor(status, "phase");
  if (phase?.startsWith("Unsafe(")) {
    return { state: "Fault", reason: "The service could not confirm release. Use the recovery controls." };
  }
  if (first(status, "enrollments") === "0") {
    return {
      state: "Inactive",
      reason: "No earphones are enrolled. Connect them, then enroll.",
      action: "enroll",
      actionLabel: "Enroll connected earphones",
    };
  }
  if (first(status, "restart_latch") === "true") {
    return {
      state: "Fault",
      reason: "The restart limit was reached. Clear the failure stop to try again.",
      action: "clear-latch",
      actionLabel: "Clear and restart",
    };
  }
  const bootOff = first(status, "stay_inactive") === "true" || first(status, "boot_disabled") === "true";
  if (bootOff || first(status, "paused") === "true") {
    if (phase === "Starting" || phase === "Running" || phase?.startsWith("Stopping ")) {
      return { state: "Discovering", reason: "Waiting for the service to release the earphone buttons." };
    }
    if (!phase?.startsWith("Inactive(") && phase !== "Latched") {
      return { state: "Fault", reason: "The off setting is saved. Release has yet to be confirmed." };
    }
    return bootOff
      ? { state: "Inactive", reason: "Kept off, including after reboot.", action: "enable-next-boot", actionLabel: "Allow blocking again" }
      : { state: "Inactive", reason: "Paused. Earphone buttons work normally.", action: "resume", actionLabel: "Resume" };
  }
  const active = activeReport(status);
  if (active) {
    const blocked = active[2] === "none" ? [] : active[2].split(",");
    if (blocked.length === 0) {
      return { state: "Ready", reason: "Earphones connected. Nothing is blocked." };
    }
    const names = blocked.map((g) => (GROUP_FROM_STATUS[g] ? label(GROUP_FROM_STATUS[g]) : g));
    const saved = first(status, "saved_generation");
    const pending = saved && saved !== active[1] ? ` Saved settings ${saved} are pending.` : "";
    return { state: "Active", reason: `Blocking ${names.join(", ")}.${pending}` };
  }
  if (savedGroups(status).length === 0) {
    return { state: "Inactive", reason: "Choose the controls to block, then save your settings." };
  }
  if (first(status, "enabled") !== "true") {
    return { state: "Inactive", reason: "Blocking is off.", action: "enable", actionLabel: "Turn blocking on" };
  }
  if (!phase) {
    return { state: "Fault", reason: "The background service is unavailable. Reboot to start it." };
  }
  if (phase !== "Running") {
    return { state: "Discovering", reason: "Waiting for the background service." };
  }
  const state = workerState(status) ?? "";
  switch (state) {
    case "waiting_accessory":
    case "disconnected":
      return { state: "Waiting for earphones", reason: "Connect the enrolled earphones." };
    case "not_neutral":
      return { state: "Discovering", reason: "Release every earphone button." };
    case "grab_conflict":
      return { state: "Fault", reason: "Another app is holding the earphones." };
    case "":
    case "disabled":
    case "discovering":
    case "recovery_inactive":
    case "stopped":
      return { state: "Discovering", reason: "Applying the saved settings." };
    default:
      return { state: "Fault", reason: `Worker reported: ${state}` };
  }
}

/** Builds a fixed command line; only allowlisted operations and group names reach the shell. */
export function command(operation: Operation): string {
  if (!OPERATIONS.includes(operation)) throw new Error("operation not allowed");
  return `${WORKER} ${operation}`;
}

export function applyCommand(groups: readonly string[] | "block-all", enable: boolean): string {
  let selection: string;
  if (groups === "block-all") {
    selection = "block-all";
  } else {
    if (groups.some((g) => !GROUPS.includes(g as GroupName)) || new Set(groups).size !== groups.length) {
      throw new Error("group not allowed");
    }
    selection = groups.length === 0 ? "passthrough" : groups.join(",");
  }
  return `${WORKER} apply ${selection}${enable ? " enable" : ""}`;
}

/** A control reply acknowledges a save or enrollment. Activation comes from status. */
export function result(operation: string, output: string): { ok: boolean; text: string; generation?: string } {
  const value = first(parse(output), operation);
  if (value === undefined) return { ok: false, text: output.trim() || "No response." };
  if (operation === "enroll") return { ok: /^ok:enrolled:[a-z0-9_]+$/.test(value), text: value };
  const generation = value.match(/^ok:saved_generation:([1-9]\d*)$/)?.[1];
  return generation ? { ok: true, text: value, generation } : { ok: false, text: value };
}
