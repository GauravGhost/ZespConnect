import { describe, expect, it } from "vitest";

import { SETTINGS_NAV, shortcutLabel, tabForShortcut } from "./navigation";

const press = (key: string, modifiers: Partial<KeyboardEvent> = {}) => ({
  key,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...modifiers,
});

describe("tabForShortcut", () => {
  it("opens tabs with Cmd on macOS", () => {
    expect(tabForShortcut(press("1", { metaKey: true }), true)).toBe("overview");
    expect(tabForShortcut(press("8", { metaKey: true }), true)).toBe("console");
    expect(tabForShortcut(press(",", { metaKey: true }), true)).toBe("settings");
  });

  it("opens tabs with Ctrl elsewhere", () => {
    expect(tabForShortcut(press("6", { ctrlKey: true }), false)).toBe("processes");
    expect(tabForShortcut(press("7", { ctrlKey: true }), false)).toBe("performance");
    expect(tabForShortcut(press("9", { ctrlKey: true }), false)).toBe("emulators");
    expect(tabForShortcut(press("0", { ctrlKey: true }), false)).toBe("wireless");
  });

  it("ignores the other platform's modifier", () => {
    expect(tabForShortcut(press("1", { ctrlKey: true }), true)).toBeNull();
    expect(tabForShortcut(press("1", { metaKey: true }), false)).toBeNull();
  });

  it("ignores presses without the modifier, with extra modifiers, or on unbound keys", () => {
    expect(tabForShortcut(press("1"), true)).toBeNull();
    expect(tabForShortcut(press("1", { metaKey: true, shiftKey: true }), true)).toBeNull();
    expect(tabForShortcut(press("1", { metaKey: true, altKey: true }), true)).toBeNull();
    expect(tabForShortcut(press("q", { metaKey: true }), true)).toBeNull();
  });
});

describe("shortcutLabel", () => {
  it("labels shortcuts for the platform", () => {
    expect(shortcutLabel(SETTINGS_NAV, true)).toBe("⌘,");
    expect(shortcutLabel(SETTINGS_NAV, false)).toBe("Ctrl+,");
  });
});
