/**
 * Enter sends in chat apps. WhatsApp and Telegram treat a hardware Enter as a new line and
 * send on Ctrl+Enter (Telegram: `ChatActivityEnterView`, when "Send by Enter" is off), so a
 * bare Enter is replayed as Ctrl+Enter. Shift+Enter is left alone and stays a new line.
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

/** A full Ctrl+Enter press as a real keyboard sends it: Ctrl down, Enter down and up, Ctrl up. */
export const SEND_STROKES: readonly KeyStroke[] = [
  { action: "down", keycode: KEYCODE_CTRL_LEFT, meta: META_CTRL_LEFT },
  { action: "down", keycode: KEYCODE_ENTER, meta: META_CTRL_LEFT },
  { action: "up", keycode: KEYCODE_ENTER, meta: META_CTRL_LEFT },
  { action: "up", keycode: KEYCODE_CTRL_LEFT, meta: 0 },
];

/** Whether a window showing `appPackage` turns a bare Enter into Send. */
export function isChatApp(appPackage: string | null): boolean {
  return appPackage !== null && CHAT_APPS.has(appPackage);
}

/** Whether `event` is Enter with no modifier held. */
export function isBareEnter(event: KeyboardEvent): boolean {
  if (event.shiftKey || event.ctrlKey || event.altKey || event.metaKey) return false;
  return ENTER_CODES.has(event.code);
}
