import { describe, expect, it } from "vitest";

import { matchesQuery, parseLogLine } from "./logcat";

const LINE =
  "11-06 10:31:01.066  3992  4124 I AmbientCueRepository: Receiving SmartSpace targets # 0";

describe("parseLogLine", () => {
  it("reads level and tag from a threadtime line", () => {
    const parsed = parseLogLine({ stream: "stdout", text: LINE });
    expect(parsed.level).toBe("I");
    expect(parsed.tag).toBe("AmbientCueRepository");
  });

  it("leaves headers and stderr without level or tag", () => {
    expect(
      parseLogLine({ stream: "stdout", text: "--------- beginning of main" }).level,
    ).toBeNull();
    const error = parseLogLine({ stream: "stderr", text: LINE });
    expect(error.stderr).toBe(true);
    expect(error.level).toBeNull();
  });
});

describe("matchesQuery", () => {
  const parsed = parseLogLine({ stream: "stdout", text: LINE });

  it("matches tag and text case-insensitively", () => {
    expect(matchesQuery(parsed, { tag: "ambientcue", search: "" })).toBe(true);
    expect(matchesQuery(parsed, { tag: "", search: "SMARTSPACE" })).toBe(true);
    expect(matchesQuery(parsed, { tag: "Other", search: "" })).toBe(false);
    expect(matchesQuery(parsed, { tag: "", search: "missing" })).toBe(false);
  });

  it("drops untagged lines once a tag filter is set", () => {
    const header = parseLogLine({ stream: "stdout", text: "--------- beginning of main" });
    expect(matchesQuery(header, { tag: "", search: "" })).toBe(true);
    expect(matchesQuery(header, { tag: "x", search: "" })).toBe(false);
  });
});
