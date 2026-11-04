import { open } from "@tauri-apps/plugin-dialog";
import type { ToolKind } from "@/types";

/** Ask the user for a tool's executable, starting at `currentPath`, or null if they cancelled. */
export function pickToolExecutable(
  kind: ToolKind,
  currentPath: string | null,
): Promise<string | null> {
  return open({
    multiple: false,
    directory: false,
    defaultPath: currentPath ?? undefined,
    title: `Select the ${kind} executable`,
  }).then((selected) => (typeof selected === "string" ? selected : null));
}
