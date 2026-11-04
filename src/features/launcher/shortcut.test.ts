import { describe, expect, it } from "vitest";

import { formatShortcut, shortcutFromKeyEvent } from "./shortcut";

function key(
  code: string,
  mods: Partial<Record<"metaKey" | "ctrlKey" | "altKey" | "shiftKey", boolean>> = {},
) {
  return { code, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...mods };
}

describe("shortcutFromKeyEvent", () => {
  it("maps Cmd on macOS and Ctrl elsewhere to CommandOrControl", () => {
    expect(shortcutFromKeyEvent(key("Space", { metaKey: true, shiftKey: true }), true)).toBe(
      "CommandOrControl+Shift+Space",
    );
    expect(shortcutFromKeyEvent(key("Space", { ctrlKey: true, shiftKey: true }), false)).toBe(
      "CommandOrControl+Shift+Space",
    );
  });

  it("keeps Ctrl on macOS and the Windows key elsewhere apart", () => {
    expect(shortcutFromKeyEvent(key("KeyK", { ctrlKey: true }), true)).toBe("Control+K");
    expect(shortcutFromKeyEvent(key("KeyK", { metaKey: true }), false)).toBe("Super+K");
  });

  it("names letter and digit keys plainly", () => {
    expect(shortcutFromKeyEvent(key("KeyK", { altKey: true }), false)).toBe("Alt+K");
    expect(shortcutFromKeyEvent(key("Digit1", { altKey: true }), false)).toBe("Alt+1");
  });

  it("rejects bare keys and shift-only combos", () => {
    expect(shortcutFromKeyEvent(key("KeyA"), false)).toBeNull();
    expect(shortcutFromKeyEvent(key("KeyA", { shiftKey: true }), false)).toBeNull();
  });

  it("returns null while only modifiers are held", () => {
    expect(
      shortcutFromKeyEvent(key("ShiftLeft", { shiftKey: true, ctrlKey: true }), false),
    ).toBeNull();
    expect(shortcutFromKeyEvent(key("MetaLeft", { metaKey: true }), true)).toBeNull();
  });
});

describe("formatShortcut", () => {
  it("formats for display", () => {
    expect(formatShortcut("CommandOrControl+Shift+Space", true)).toBe("Cmd+Shift+Space");
    expect(formatShortcut("CommandOrControl+Shift+Space", false)).toBe("Ctrl+Shift+Space");
    expect(formatShortcut("Alt+K", true)).toBe("Option+K");
    expect(formatShortcut("Super+K", false)).toBe("Win+K");
  });
});
