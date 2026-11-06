import type { SendOutcome } from "@/types";

/** Shown to the user; matches `services::files::DROP_DIR` on the phone. */
export const DROP_FOLDER = "Download/ZespConnect";

export interface DropResult {
  name: string;
  outcome?: SendOutcome;
  error?: string;
}

/** The last segment of a Windows or POSIX path. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

/** Mirrors `services::files::is_apk`: the backend installs these instead of copying them. */
export function isApk(path: string): boolean {
  return /\.(apk|apks|xapk)$/i.test(path);
}

const plural = (count: number, word: string) => `${count} ${word}${count === 1 ? "" : "s"}`;

/** One line describing a finished batch of dropped files. */
export function dropSummary(results: DropResult[]): { text: string; failed: boolean } {
  const failures = results.filter((result) => result.error !== undefined);
  if (failures.length > 0) {
    const [first] = failures;
    const prefix =
      results.length === 1 ? "Could not send" : `${failures.length} of ${results.length} failed.`;
    return { text: `${prefix} ${first.name}: ${first.error}`, failed: true };
  }

  const installed = results.filter((result) => result.outcome === "installed");
  const copied = results.filter((result) => result.outcome === "copied");
  const parts: string[] = [];
  if (installed.length === 1) parts.push(`Installed ${installed[0].name}`);
  else if (installed.length > 1) parts.push(`Installed ${plural(installed.length, "app")}`);
  if (copied.length === 1) parts.push(`sent ${copied[0].name} to ${DROP_FOLDER}`);
  else if (copied.length > 1) parts.push(`sent ${plural(copied.length, "file")} to ${DROP_FOLDER}`);

  const text = parts.join(", ");
  return { text: text.charAt(0).toUpperCase() + text.slice(1), failed: false };
}
