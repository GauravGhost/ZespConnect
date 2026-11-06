import { describe, expect, it } from "vitest";

import { MAX_OUTPUT_CHARS, capOutput, cleanTerminalText } from "./shell";

describe("cleanTerminalText", () => {
  it("turns terminal line endings into plain newlines", () => {
    expect(cleanTerminalText("pwd\r\r\r\n/sdcard\r\r\nshiba:/sdcard $ ")).toBe(
      "pwd\n/sdcard\nshiba:/sdcard $ ",
    );
  });

  it("drops colors and other escape sequences", () => {
    expect(cleanTerminalText("\x1b[1;34mDownload\x1b[0m  \x1b]0;title\x07file\x07")).toBe(
      "Download  file",
    );
  });

  it("keeps tabs", () => {
    expect(cleanTerminalText("a\tb")).toBe("a\tb");
  });
});

describe("capOutput", () => {
  it("leaves short output alone", () => {
    expect(capOutput("one\ntwo")).toBe("one\ntwo");
  });

  it("drops the oldest output at a line break", () => {
    const capped = capOutput("x".repeat(MAX_OUTPUT_CHARS) + "\n" + "y".repeat(10));
    expect(capped).toBe("y".repeat(10));
  });
});
