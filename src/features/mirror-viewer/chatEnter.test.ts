import { describe, expect, it } from "vitest";

import { isBareEnter, isChatApp, SEND_STROKES } from "./chatEnter";

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

describe("isBareEnter", () => {
  it("accepts Enter and the keypad Enter", () => {
    expect(isBareEnter(enter())).toBe(true);
    expect(isBareEnter(enter({ code: "NumpadEnter" }))).toBe(true);
  });

  it("leaves Shift+Enter as a new line", () => {
    expect(isBareEnter(enter({ shiftKey: true }))).toBe(false);
  });

  it("leaves other modified Enters and other keys alone", () => {
    expect(isBareEnter(enter({ ctrlKey: true }))).toBe(false);
    expect(isBareEnter(enter({ altKey: true }))).toBe(false);
    expect(isBareEnter(enter({ metaKey: true }))).toBe(false);
    expect(isBareEnter(new KeyboardEvent("keydown", { code: "KeyA", key: "a" }))).toBe(false);
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
