/** Escape sequences: CSI (colors, cursor moves), OSC (window titles) and two-byte escapes. */
// eslint-disable-next-line no-control-regex
const ESCAPES = /\x1b\[[0-?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)?|\x1b[@-_]/g;
// The device terminal ends lines with "\r\r\n"; other control bytes (bell, backspace) are noise here.
// eslint-disable-next-line no-control-regex
const CONTROL = /[\r\x00-\x08\x0b-\x1f\x7f]/g;

/** Keeps a long session's transcript light; older output is dropped at a line break. */
export const MAX_OUTPUT_CHARS = 200_000;

/** Text from the device terminal as plain lines: no colors, cursor codes or carriage returns. */
export function cleanTerminalText(text: string): string {
  return text.replace(ESCAPES, "").replace(CONTROL, "");
}

export function capOutput(output: string): string {
  if (output.length <= MAX_OUTPUT_CHARS) return output;
  const tail = output.slice(output.length - MAX_OUTPUT_CHARS);
  const lineStart = tail.indexOf("\n");
  return lineStart === -1 ? tail : tail.slice(lineStart + 1);
}
