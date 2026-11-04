/**
 * Enter sends in chat apps. WhatsApp and Telegram treat a hardware Enter as a new line and
 * send on Ctrl+Enter (Telegram: `ChatActivityEnterView`, when "Send by Enter" is off), so a
 * bare Enter is replayed as Ctrl+Enter. WhatsApp also sends on Shift+Enter, so Shift+Enter
 * is replayed as a bare Enter, which is a new line in both apps.
 */

import type { KeyAction } from "@/types";
import { META_CTRL_LEFT } from "./keymap";

const CHAT_APPS = new Set([
  "com.whatsapp",
  "com.whatsapp.w4b",
  "org.telegram.messenger",
  "org.telegram.messenger.web",
]);

const KEYCODE_ENTER = 66;
const KEYCODE_CTRL_LEFT = 113;
const ENTER_CODES = new Set(["Enter", "NumpadEnter"]);

export interface KeyStroke {
  action: KeyAction;
  keycode: number;
  meta: number;
}

/** A bare Enter press, without the Shift the user holds. */
export const NEW_LINE_STROKES: readonly KeyStroke[] = [
  { action: "down", keycode: KEYCODE_ENTER, meta: 0 },
  { action: "up", keycode: KEYCODE_ENTER, meta: 0 },
];

/** A full Ctrl+Enter press as a real keyboard sends it: Ctrl down, Enter down and up, Ctrl up. */
export const SEND_STROKES: readonly KeyStroke[] = [
  { action: "down", keycode: KEYCODE_CTRL_LEFT, meta: META_CTRL_LEFT },
  { action: "down", keycode: KEYCODE_ENTER, meta: META_CTRL_LEFT },
  { action: "up", keycode: KEYCODE_ENTER, meta: META_CTRL_LEFT },
  { action: "up", keycode: KEYCODE_CTRL_LEFT, meta: 0 },
];

/** Whether a window showing `appPackage` remaps Enter and Shift+Enter (see `chatEnterStrokes`). */
export function isChatApp(appPackage: string | null): boolean {
  return appPackage !== null && CHAT_APPS.has(appPackage);
}

/**
 * What to send for `event` in a chat app's window: Send for a bare Enter, a new line for
 * Shift+Enter, or null to forward `event` unchanged.
 */
export function chatEnterStrokes(event: KeyboardEvent): readonly KeyStroke[] | null {
  if (!ENTER_CODES.has(event.code)) return null;
  if (event.ctrlKey || event.altKey || event.metaKey) return null;
  return event.shiftKey ? NEW_LINE_STROKES : SEND_STROKES;
}
