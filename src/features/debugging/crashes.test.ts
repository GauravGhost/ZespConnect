import { describe, expect, it } from "vitest";

import type { CrashReport } from "@/types";
import { groupByApp } from "./crashes";

function report(app: string, time: string): CrashReport {
  return {
    kind: "crash",
    source: "data_app_crash",
    time,
    app,
    summary: "java.lang.IllegalStateException",
    details: "",
    truncated: false,
  };
}

describe("groupByApp", () => {
  it("keeps newest-first order within and across apps", () => {
    const reports = [
      report("com.b", "2026-11-06 10:00:00"),
      report("com.a", "2026-11-05 10:00:00"),
      report("com.b", "2026-11-04 10:00:00"),
    ];
    expect(groupByApp(reports)).toEqual([
      { app: "com.b", reports: [reports[0], reports[2]] },
      { app: "com.a", reports: [reports[1]] },
    ]);
  });
});
