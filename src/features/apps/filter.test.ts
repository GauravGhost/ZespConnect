import { describe, expect, it } from "vitest";

import type { AppInfo } from "@/types";
import { filterApps } from "./filter";

function makeApp(overrides: Partial<AppInfo> = {}): AppInfo {
  return {
    package: "com.example.app",
    label: "Example",
    is_system: false,
    version_name: null,
    label_source: "scrcpy",
    icon_path: null,
    ...overrides,
  };
}

const apps = [
  makeApp({ package: "com.example.notes", label: "Notes" }),
  makeApp({ package: "com.android.settings", label: "Settings", is_system: true }),
  makeApp({ package: "com.google.android.apps.maps", label: "Google Maps" }),
];

describe("filterApps", () => {
  it("filters by user/system/all scope", () => {
    expect(filterApps(apps, "", "user").map((a) => a.label)).toEqual(["Notes", "Google Maps"]);
    expect(filterApps(apps, "", "system").map((a) => a.label)).toEqual(["Settings"]);
    expect(filterApps(apps, "", "all")).toHaveLength(3);
  });

  it("matches on name or package, case-insensitively", () => {
    expect(filterApps(apps, "maps", "all").map((a) => a.package)).toEqual([
      "com.google.android.apps.maps",
    ]);
    expect(filterApps(apps, "ANDROID.SETTINGS", "all").map((a) => a.label)).toEqual(["Settings"]);
  });

  it("combines scope and query", () => {
    expect(filterApps(apps, "settings", "user")).toEqual([]);
    expect(filterApps(apps, "settings", "system").map((a) => a.label)).toEqual(["Settings"]);
  });
});
