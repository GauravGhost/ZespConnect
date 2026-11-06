import { describe, expect, it } from "vitest";

import { apiLevel, atLeastApi } from "./android";

describe("apiLevel", () => {
  it("parses a reported level", () => {
    expect(apiLevel("33")).toBe(33);
  });

  it("is null when unknown", () => {
    expect(apiLevel(null)).toBeNull();
    expect(apiLevel("")).toBeNull();
  });
});

describe("atLeastApi", () => {
  it("compares against the minimum", () => {
    expect(atLeastApi("33", 33)).toBe(true);
    expect(atLeastApi("31", 33)).toBe(false);
  });

  it("treats an unknown version as too old", () => {
    expect(atLeastApi(undefined, 33)).toBe(false);
  });
});
