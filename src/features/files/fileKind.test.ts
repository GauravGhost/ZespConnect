import { describe, expect, it } from "vitest";

import { fileKind } from "./fileKind";

describe("fileKind", () => {
  it("classifies common media by extension, ignoring case", () => {
    expect(fileKind("IMG_2024.JPG")).toBe("image");
    expect(fileKind("clip.mp4")).toBe("video");
    expect(fileKind("voice note.m4a")).toBe("audio");
    expect(fileKind("report.pdf")).toBe("document");
    expect(fileKind("app-release.apk")).toBe("app");
  });

  it("uses only the last extension", () => {
    expect(fileKind("backup.tar.gz")).toBe("archive");
  });

  it("falls back to other for unknown, missing or dotfile-only names", () => {
    expect(fileKind("data.bin")).toBe("other");
    expect(fileKind("README")).toBe("other");
    expect(fileKind(".nomedia")).toBe("other");
  });
});
