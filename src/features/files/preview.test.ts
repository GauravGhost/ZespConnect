import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string, protocol: string) =>
    `${protocol}://localhost/${encodeURIComponent(path)}`,
}));

import { deviceFileUrl, previewKind, TEXT_PREVIEW_LIMIT } from "./preview";

describe("previewKind", () => {
  it("previews media, PDFs and text in the app", () => {
    expect(previewKind("IMG_1.JPG", 10)).toBe("image");
    expect(previewKind("clip.mov", 10)).toBe("video");
    expect(previewKind("song.mp3", 10)).toBe("audio");
    expect(previewKind("scan.pdf", 10)).toBe("pdf");
    expect(previewKind("notes.txt", 10)).toBe("text");
  });

  it("leaves other formats and large text files to the default app", () => {
    expect(previewKind("movie.mkv", 10)).toBeNull();
    expect(previewKind("archive.zip", 10)).toBeNull();
    expect(previewKind("README", 10)).toBeNull();
    expect(previewKind("huge.log", TEXT_PREVIEW_LIMIT + 1)).toBeNull();
  });
});

describe("deviceFileUrl", () => {
  it("encodes the serial and device path into one address", () => {
    expect(deviceFileUrl("192.168.1.5:5555", "/sdcard/My Photo.jpg")).toBe(
      "devicefile://localhost/192.168.1.5%3A5555%2Fsdcard%2FMy%20Photo.jpg",
    );
  });
});
