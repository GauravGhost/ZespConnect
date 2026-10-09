/**
 * Keyboard and mouse translation for desktop-mode windows: browser
 * `KeyboardEvent.code`s to Android `KEYCODE_*`s, modifier state to Android's
 * `META_*` mask, and mouse buttons to `MotionEvent.BUTTON_*`.
 *
 * `code` is the physical key, so shortcuts work whatever the keyboard layout;
 * printable characters are sent as text instead (see `isTextKey`) so layouts,
 * accents and emoji come through as typed.
 */

const IS_MAC = /Mac|iPhone|iPad/.test(navigator.platform);

const KEYCODES: Record<string, number> = {
  Enter: 66,
  NumpadEnter: 160,
  Backspace: 67,
  Delete: 112,
  Tab: 61,
  Escape: 111,
  Space: 62,
  ArrowUp: 19,
  ArrowDown: 20,
  ArrowLeft: 21,
  ArrowRight: 22,
  Home: 122,
  End: 123,
  PageUp: 92,
  PageDown: 93,
  Insert: 124,
  ShiftLeft: 59,
  ShiftRight: 60,
  ControlLeft: 113,
  ControlRight: 114,
  AltLeft: 57,
  AltRight: 58,
  CapsLock: 115,
  Minus: 69,
  Equal: 70,
  BracketLeft: 71,
  BracketRight: 72,
  Backslash: 73,
  Semicolon: 74,
  Quote: 75,
  Slash: 76,
  Comma: 55,
  Period: 56,
  Backquote: 68,
  // On a Mac, ⌘ acts as Ctrl so ⌘C / ⌘V / ⌘A / ⌘Z do what they do on the Mac.
  MetaLeft: IS_MAC ? 113 : 117,
  MetaRight: IS_MAC ? 114 : 118,
};

// KeyA-KeyZ → KEYCODE_A (29) … KEYCODE_Z (54).
for (let i = 0; i < 26; i++) KEYCODES[`Key${String.fromCharCode(65 + i)}`] = 29 + i;
// Digit0-Digit9 → KEYCODE_0 (7) … KEYCODE_9 (16).
for (let i = 0; i < 10; i++) KEYCODES[`Digit${i}`] = 7 + i;
// F1-F12 → KEYCODE_F1 (131) … KEYCODE_F12 (142).
for (let i = 1; i <= 12; i++) KEYCODES[`F${i}`] = 130 + i;

/** The Android keycode for a physical key, if it has one. */
export function androidKeycode(event: KeyboardEvent): number | undefined {
  return KEYCODES[event.code];
}

const META_SHIFT_ON = 0x1;
const META_SHIFT_LEFT_ON = 0x40;
const META_ALT_ON = 0x2;
const META_ALT_LEFT_ON = 0x10;
const META_CTRL_ON = 0x1000;
const META_CTRL_LEFT_ON = 0x2000;
const META_META_ON = 0x10000;
const META_META_LEFT_ON = 0x20000;
const META_CAPS_LOCK_ON = 0x100000;

/** Whether Ctrl is effectively held (⌘ counts as Ctrl on a Mac). */
function ctrlHeld(event: KeyboardEvent): boolean {
  return event.ctrlKey || (IS_MAC && event.metaKey);
}

/** The Android `META_*` mask for the modifiers held during `event`. */
export function androidMetaState(event: KeyboardEvent): number {
  let meta = 0;
  if (event.shiftKey) meta |= META_SHIFT_ON | META_SHIFT_LEFT_ON;
  if (event.altKey) meta |= META_ALT_ON | META_ALT_LEFT_ON;
  if (ctrlHeld(event)) meta |= META_CTRL_ON | META_CTRL_LEFT_ON;
  if (!IS_MAC && event.metaKey) meta |= META_META_ON | META_META_LEFT_ON;
  if (event.getModifierState("CapsLock")) meta |= META_CAPS_LOCK_ON;
  return meta;
}

/**
 * Whether `event` types a character that should be sent as text: a single
 * printable character with no shortcut modifier held (Shift is part of the
 * character). Shortcuts like Ctrl+C go as keycodes instead.
 */
export function isTextKey(event: KeyboardEvent): boolean {
  if (ctrlHeld(event) || event.altKey || event.metaKey) return false;
  return [...event.key].length === 1;
}

/** Browser `MouseEvent.button` → Android `MotionEvent.BUTTON_*`. */
const BUTTONS: Record<number, number> = {
  0: 1, // primary
  1: 4, // middle → tertiary
  2: 2, // secondary
  3: 8, // back
  4: 16, // forward
};

/** The Android button for a browser `MouseEvent.button` (0 if unknown). */
export function androidButton(button: number): number {
  return BUTTONS[button] ?? 0;
}

/**
 * The Android buttons mask for a browser `MouseEvent.buttons` mask. Its bits
 * (primary 1, secondary 2, middle 4, back 8, forward 16) already match Android's.
 */
export function androidButtons(buttons: number): number {
  return buttons & 0x1f;
}

/** Pixels of wheel travel per notch (a mouse wheel click is ~100px). */
const PIXELS_PER_NOTCH = 100;
/** Lines per notch when the wheel reports lines. */
const LINES_PER_NOTCH = 3;

/** A wheel event's travel in notches, `[horizontal, vertical]`, browser axes. */
export function wheelNotches(event: WheelEvent): [number, number] {
  const unit =
    event.deltaMode === WheelEvent.DOM_DELTA_LINE
      ? LINES_PER_NOTCH
      : event.deltaMode === WheelEvent.DOM_DELTA_PAGE
        ? 1
        : PIXELS_PER_NOTCH;
  return [event.deltaX / unit, event.deltaY / unit];
}
