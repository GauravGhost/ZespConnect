const STORAGE_KEY = "zesp-connect-launcher-recents";
const MAX_RECENTS = 5;

type RecentsBySerial = Record<string, string[]>;

/** Packages recently opened from the launcher on `serial`, most recent first. */
export function readRecents(serial: string): string[] {
  return readAll()[serial] ?? [];
}

export function rememberRecent(serial: string, pkg: string): void {
  const all = readAll();
  const rest = (all[serial] ?? []).filter((existing) => existing !== pkg);
  all[serial] = [pkg, ...rest].slice(0, MAX_RECENTS);
  localStorage.setItem(STORAGE_KEY, JSON.stringify(all));
}

function readAll(): RecentsBySerial {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}");
    return parsed && typeof parsed === "object" ? (parsed as RecentsBySerial) : {};
  } catch {
    return {};
  }
}
