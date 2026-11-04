import { describe, expect, it } from "vitest";

import type { AppInfo } from "@/types";
import { rankLauncherApps } from "./rank";

function makeApp(label: string): AppInfo {
  return {
    package: `pkg.${label.toLowerCase().replace(/\s+/g, "")}`,
    label,
    is_system: false,
    version_name: null,
    label_source: "scrcpy",
    icon_path: null,
  };
}

const labels = (apps: AppInfo[]) => apps.map((app) => app.label);

describe("rankLauncherApps", () => {
  const apps = [makeApp("Alpha"), makeApp("Beta"), makeApp("Chrome")];

  it("puts recents first, then the rest alphabetically, when the query is empty", () => {
    expect(labels(rankLauncherApps(apps, "", ["pkg.chrome"]))).toEqual(["Chrome", "Alpha", "Beta"]);
  });

  it("ignores recents that are no longer installed", () => {
    expect(labels(rankLauncherApps(apps, "", ["pkg.gone", "pkg.beta"]))).toEqual([
      "Beta",
      "Alpha",
      "Chrome",
    ]);
  });

  it("puts label-prefix matches before other matches", () => {
    const chat = [makeApp("Call Whisper"), makeApp("WhatsApp")];
    expect(labels(rankLauncherApps(chat, "wh", []))).toEqual(["WhatsApp", "Call Whisper"]);
  });

  it("returns nothing when no app matches", () => {
    expect(rankLauncherApps(apps, "zzz", [])).toEqual([]);
  });
});
