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
  return runtime(status, "worker", "active_generation") ?? "0";
}

export function supervisor(status: Status, field: string): string | undefined {
  return runtime(status, "supervisor", field);
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
      reason: "Stopped after repeated failures. Earphone buttons work normally.",
      action: "clear-latch",
      actionLabel: "Clear and restart",
    };
  }
  if (first(status, "stay_inactive") === "true" || first(status, "boot_disabled") === "true") {
    return {
      state: "Inactive",
      reason: "Kept off, including after reboot.",
      action: "enable-next-boot",
      actionLabel: "Allow at next boot",
    };
  }
  if (first(status, "paused") === "true") {
    return { state: "Inactive", reason: "Paused. Earphone buttons work normally.", action: "resume", actionLabel: "Resume" };
  }
  if (first(status, "enabled") !== "true") {
    return { state: "Inactive", reason: "Blocking is off.", action: "enable", actionLabel: "Turn blocking on" };
  }
  const phase = supervisor(status, "phase");
  if (!phase || phase === "none") {
    return { state: "Fault", reason: "The background service is not running. Reboot to start it." };
  }
  const state = workerState(status) ?? "";
  const active = state.match(/^active:generation=(\d+):blocked=(.+)$/);
  if (active) {
    const blocked = active[2] === "none" ? [] : active[2].split(",");
    if (blocked.length === 0) {
      return { state: "Ready", reason: "Earphones connected. Nothing is blocked." };
    }
    const names = blocked.map((g) => (GROUP_FROM_STATUS[g] ? label(GROUP_FROM_STATUS[g]) : g));
    return { state: "Active", reason: `Blocking ${names.join(", ")}.` };
  }
  switch (state) {
    case "waiting_accessory":
      return { state: "Waiting for earphones", reason: "Connect the enrolled earphones." };
    case "not_neutral":
      return { state: "Waiting for earphones", reason: "Release every earphone button." };
    case "grab_conflict":
      return { state: "Fault", reason: "Another app is holding the earphones." };
    case "":
    case "disabled":
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

/** The `<operation>=ok:...` or `=error:...` result line, or the whole output. */
export function result(operation: string, output: string): { ok: boolean; text: string } {
  const value = first(parse(output), operation);
  if (value === undefined) return { ok: false, text: output.trim() || "No response." };
  return { ok: value.startsWith("ok"), text: value };
}
