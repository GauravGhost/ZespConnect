import type { AppInfo } from "@/types";

export type AppFilter = "user" | "system" | "all";

/** Filter the loaded app list by user/system scope and a name-or-package query. */
export function filterApps(apps: AppInfo[], query: string, filter: AppFilter): AppInfo[] {
  const needle = query.trim().toLowerCase();

  return apps.filter((app) => {
    if (filter === "user" && app.is_system) return false;
    if (filter === "system" && !app.is_system) return false;
    if (!needle) return true;
    return app.label.toLowerCase().includes(needle) || app.package.toLowerCase().includes(needle);
  });
}
