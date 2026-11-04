import { beforeEach, describe, expect, it } from "vitest";

import { readRecents, rememberRecent } from "./recents";

const STORAGE_KEY = "zesp-connect-launcher-recents";

describe("launcher recents", () => {
  beforeEach(() => localStorage.clear());

  it("remembers per serial, most recent first, without duplicates", () => {
    rememberRecent("phone", "a");
    rememberRecent("phone", "b");
    rememberRecent("phone", "a");
    rememberRecent("other", "c");
    expect(readRecents("phone")).toEqual(["a", "b"]);
    expect(readRecents("other")).toEqual(["c"]);
  });

  it("keeps at most 5", () => {
    for (const pkg of ["1", "2", "3", "4", "5", "6"]) rememberRecent("phone", pkg);
    expect(readRecents("phone")).toEqual(["6", "5", "4", "3", "2"]);
  });

  it("returns [] for unreadable storage", () => {
    localStorage.setItem(STORAGE_KEY, "not json");
    expect(readRecents("phone")).toEqual([]);
  });
});
