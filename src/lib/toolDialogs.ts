import { open } from "@tauri-apps/plugin-dialog";
import type { ToolKind } from "@/types";

/** Ask the user for a tool's executable, or null if they cancelled. */
export function pickToolExecutable(kind: ToolKind): Promise<string | null> {
  return open({
    multiple: false,
    directory: false,
    title: `Select the ${kind} executable`,
  }).then((selected) => (typeof selected === "string" ? selected : null));
}
