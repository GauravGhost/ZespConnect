import { beforeEach, describe, expect, it } from "vitest";

import { CAPTURE_STORAGE_KEY, readCaptureSettings } from "./capture";

const DEFAULTS = {
  screenshotTarget: "clipboard",
  recordingTarget: "folder",
  screenshotDir: null,
  recordingDir: null,
};

describe("readCaptureSettings", () => {
  beforeEach(() => localStorage.clear());

  it("defaults", () => {
    expect(readCaptureSettings()).toEqual(DEFAULTS);
  });

  it("reads what another window wrote", () => {
    localStorage.setItem(
      CAPTURE_STORAGE_KEY,
      JSON.stringify({
        state: {
          screenshotTarget: "phone",
          recordingTarget: "phone",
          screenshotDir: "D:/shots",
          recordingDir: null,
        },
        version: 1,
      }),
    );
    const settings = readCaptureSettings();
    expect(settings.screenshotTarget).toBe("phone");
    expect(settings.screenshotDir).toBe("D:/shots");
  });

  it("ignores garbage", () => {
    localStorage.setItem(CAPTURE_STORAGE_KEY, "{");
    expect(readCaptureSettings()).toEqual(DEFAULTS);
  });
});
