type KeyCombo = Pick<KeyboardEvent, "code" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">;

const MODIFIER_CODES = /^(Shift|Control|Alt|Meta|OS)(Left|Right)?$/;

/**
 * The hotkey string the backend registers for a key press, or `null` when the press cannot be a
 * global hotkey. Shift alone does not count as a modifier, or the hotkey would swallow typing.
 */
export function shortcutFromKeyEvent(event: KeyCombo, isMac: boolean): string | null {
  if (MODIFIER_CODES.test(event.code)) return null;

  const primary = isMac ? event.metaKey : event.ctrlKey;
  const secondary = isMac ? event.ctrlKey : event.metaKey;
  const modifiers = [
    primary && "CommandOrControl",
    isMac && secondary && "Control",
    !isMac && secondary && "Super",
    event.altKey && "Alt",
  ].filter((modifier): modifier is string => Boolean(modifier));
  if (modifiers.length === 0) return null;

  if (event.shiftKey) modifiers.push("Shift");
  return [...modifiers, keyName(event.code)].join("+");
}

function keyName(code: string): string {
  return code.replace(/^Key(?=[A-Z]$)/, "").replace(/^Digit(?=\d$)/, "");
}

const MAC_NAMES: Record<string, string> = {
  CommandOrControl: "Cmd",
  Control: "Ctrl",
  Super: "Cmd",
  Alt: "Option",
};
const OTHER_NAMES: Record<string, string> = {
  CommandOrControl: "Ctrl",
  Control: "Ctrl",
  Super: "Win",
};

/** A hotkey string as the user's keyboard labels it. */
export function formatShortcut(shortcut: string, isMac: boolean): string {
  const names = isMac ? MAC_NAMES : OTHER_NAMES;
  return shortcut
    .split("+")
    .map((part) => names[part] ?? part)
    .join("+");
}
