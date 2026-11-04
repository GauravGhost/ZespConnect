import type { MirrorAction, MirrorKind, MirrorSession } from "@/types";

/** A control in the bottom bar; Back sits in the title bar and the torch is a toggle. */
export type MenuAction = Exclude<MirrorAction, "back" | "torch-on" | "torch-off">;

/** Controls for the main display; a virtual display has no panels to open or screen to turn. */
const MAIN_DISPLAY_ACTIONS: MenuAction[] = [
  "notifications",
  "quick-settings",
  "collapse-panels",
  "rotate",
];

const MAIN_DISPLAY_KINDS: MirrorKind[] = ["mirror", "screen-app"];

/** Whether a session's window has a Back button; a camera or desktop has nothing to go back from. */
export function offersBack(kind: MirrorKind): boolean {
  return kind === "app" || MAIN_DISPLAY_KINDS.includes(kind);
}

/** The bottom-bar controls a session offers. */
export function menuActions(session: Pick<MirrorSession, "kind" | "cameraControls">): MenuAction[] {
  if (MAIN_DISPLAY_KINDS.includes(session.kind)) return MAIN_DISPLAY_ACTIONS;
  if (session.kind === "camera" && session.cameraControls) return ["zoom-in", "zoom-out"];
  return [];
}
