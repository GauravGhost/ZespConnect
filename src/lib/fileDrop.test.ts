import { describe, expect, it } from "vitest";

import { dropSummary, fileName, isApk } from "@/lib/fileDrop";

describe("fileName", () => {
  it("takes the last segment of Windows and POSIX paths", () => {
    expect(fileName("C:\\Users\\me\\photo.jpg")).toBe("photo.jpg");
    expect(fileName("/home/me/Holiday/")).toBe("Holiday");
  });
});

describe("isApk", () => {
  it("matches the extension case-insensitively", () => {
    expect(isApk("C:\\Downloads\\App.APK")).toBe(true);
    expect(isApk("/home/me/photo.jpg")).toBe(false);
    expect(isApk("/home/me/apk")).toBe(false);
    expect(isApk("C:\\Downloads\\app.apks")).toBe(true);
    expect(isApk("/home/me/game.XAPK")).toBe(true);
  });
});

describe("dropSummary", () => {
  it("names a single file", () => {
    expect(dropSummary([{ name: "photo.jpg", outcome: "copied" }])).toEqual({
      text: "Sent photo.jpg to Download/ZespConnect",
      failed: false,
    });
    expect(dropSummary([{ name: "app.apk", outcome: "installed" }]).text).toBe("Installed app.apk");
  });

  it("counts a mixed batch", () => {
    const summary = dropSummary([
      { name: "a.apk", outcome: "installed" },
      { name: "b.jpg", outcome: "copied" },
      { name: "c.jpg", outcome: "copied" },
    ]);
    expect(summary.text).toBe("Installed a.apk, sent 2 files to Download/ZespConnect");
  });

  it("reports the first failure", () => {
    const summary = dropSummary([
      { name: "a.jpg", outcome: "copied" },
      { name: "b.jpg", error: "device offline" },
    ]);
    expect(summary).toEqual({ text: "1 of 2 failed. b.jpg: device offline", failed: true });
    expect(dropSummary([{ name: "x.apk", error: "INSTALL_FAILED" }]).text).toBe(
      "Could not send x.apk: INSTALL_FAILED",
    );
  });
});
