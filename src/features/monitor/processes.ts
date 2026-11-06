import type { ProcessInfo } from "@/types";

export type ProcessSortKey = "name" | "pid" | "cpu" | "memory";
export type ProcessScope = "apps" | "all";

export interface ProcessSort {
  key: ProcessSortKey;
  descending: boolean;
}

/** What a row reads as: the app's name when it is an app, else the command. */
export function processTitle(process: ProcessInfo): string {
  return process.label ?? process.name;
}

const COMPARE: Record<ProcessSortKey, (a: ProcessInfo, b: ProcessInfo) => number> = {
  name: (a, b) => processTitle(a).localeCompare(processTitle(b)),
  pid: (a, b) => a.pid - b.pid,
  cpu: (a, b) => a.cpu - b.cpu,
  memory: (a, b) => a.resKb - b.resKb,
};

export function visibleProcesses(
  processes: ProcessInfo[],
  query: string,
  scope: ProcessScope,
  sort: ProcessSort,
): ProcessInfo[] {
  const needle = query.trim().toLowerCase();
  const rows = processes.filter((process) => {
    if (scope === "apps" && !process.package) return false;
    if (!needle) return true;
    return (
      processTitle(process).toLowerCase().includes(needle) ||
      process.name.toLowerCase().includes(needle) ||
      String(process.pid) === needle
    );
  });
  const compare = COMPARE[sort.key];
  // Ties fall back to the pid so rows do not shuffle between refreshes.
  return rows.sort((a, b) => {
    const order = compare(a, b) || a.pid - b.pid;
    return sort.descending ? -order : order;
  });
}

/** Sorting by a new column starts with the order people want first: names A-Z, numbers high-low. */
export function nextSort(current: ProcessSort, key: ProcessSortKey): ProcessSort {
  if (current.key === key) return { key, descending: !current.descending };
  return { key, descending: key !== "name" };
}
