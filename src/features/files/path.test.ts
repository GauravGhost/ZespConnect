import { describe, expect, it } from "vitest";

import {
  baseName,
  breadcrumbs,
  breadcrumbsWithin,
  joinPath,
  parentPath,
  parentWithin,
} from "./path";

describe("joinPath", () => {
  it("adds one separator", () => {
    expect(joinPath("/sdcard", "DCIM")).toBe("/sdcard/DCIM");
    expect(joinPath("/sdcard/", "DCIM")).toBe("/sdcard/DCIM");
  });
});

describe("parentPath", () => {
  it("walks up without escaping the root", () => {
    expect(parentPath("/sdcard/Download")).toBe("/sdcard");
    expect(parentPath("/sdcard/")).toBe("/");
    expect(parentPath("/sdcard")).toBe("/");
    expect(parentPath("/")).toBe("/");
  });
});

describe("parentWithin", () => {
  it("never climbs above the root", () => {
    expect(parentWithin("/sdcard/Download", "/sdcard")).toBe("/sdcard");
    expect(parentWithin("/sdcard", "/sdcard")).toBe("/sdcard");
    expect(parentWithin("/sdcard/DCIM/Camera", "/sdcard")).toBe("/sdcard/DCIM");
  });
});

describe("breadcrumbs", () => {
  it("builds cumulative segments", () => {
    expect(breadcrumbs("/sdcard/Download")).toEqual([
      { label: "sdcard", path: "/sdcard" },
      { label: "Download", path: "/sdcard/Download" },
    ]);
    expect(breadcrumbs("/")).toEqual([]);
  });
});

describe("breadcrumbsWithin", () => {
  it("drops the root instead of repeating it", () => {
    expect(breadcrumbsWithin("/sdcard/.trash-storage/DCIM", "/sdcard")).toEqual([
      { label: ".trash-storage", path: "/sdcard/.trash-storage" },
      { label: "DCIM", path: "/sdcard/.trash-storage/DCIM" },
    ]);
    expect(breadcrumbsWithin("/sdcard", "/sdcard")).toEqual([]);
  });
});

describe("baseName", () => {
  it("handles host separators", () => {
    expect(baseName("/home/me/photo.png")).toBe("photo.png");
    expect(baseName("C:\\Users\\me\\photo.png")).toBe("photo.png");
  });
});
