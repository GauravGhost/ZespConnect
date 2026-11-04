import { describe, expect, it } from "vitest";

import { chatEnterStrokes, isChatApp, NEW_LINE_STROKES, SEND_STROKES } from "./chatEnter";

const enter = (init: KeyboardEventInit = {}) =>
  new KeyboardEvent("keydown", { code: "Enter", key: "Enter", ...init });

describe("isChatApp", () => {
  it("covers WhatsApp and Telegram only", () => {
    expect(isChatApp("com.whatsapp")).toBe(true);
    expect(isChatApp("com.whatsapp.w4b")).toBe(true);
    expect(isChatApp("org.telegram.messenger")).toBe(true);
    expect(isChatApp("com.android.chrome")).toBe(false);
    expect(isChatApp(null)).toBe(false);
  });
});

describe("chatEnterStrokes", () => {
  it("sends on Enter and the keypad Enter", () => {
    expect(chatEnterStrokes(enter())).toBe(SEND_STROKES);
    expect(chatEnterStrokes(enter({ code: "NumpadEnter" }))).toBe(SEND_STROKES);
  });

  it("turns Shift+Enter into a new line, since WhatsApp sends on it", () => {
    expect(chatEnterStrokes(enter({ shiftKey: true }))).toBe(NEW_LINE_STROKES);
  });

  it("leaves other modified Enters and other keys alone", () => {
    expect(chatEnterStrokes(enter({ ctrlKey: true }))).toBeNull();
    expect(chatEnterStrokes(enter({ altKey: true }))).toBeNull();
    expect(chatEnterStrokes(enter({ metaKey: true }))).toBeNull();
    expect(chatEnterStrokes(new KeyboardEvent("keydown", { code: "KeyA", key: "a" }))).toBeNull();
  });
});

describe("NEW_LINE_STROKES", () => {
  it("presses Enter with no modifier", () => {
    expect(NEW_LINE_STROKES).toEqual([
      { action: "down", keycode: 66, meta: 0 },
      { action: "up", keycode: 66, meta: 0 },
    ]);
  });
});

describe("SEND_STROKES", () => {
  it("presses Enter while Ctrl is held and releases everything", () => {
    expect(SEND_STROKES.map(({ action, keycode }) => `${action}:${keycode}`)).toEqual([
      "down:113",
      "down:66",
      "up:66",
      "up:113",
    ]);
    expect(SEND_STROKES[SEND_STROKES.length - 1].meta).toBe(0);
  });
});
