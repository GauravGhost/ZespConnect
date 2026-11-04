import { filterApps } from "@/features/apps/filter";
import type { AppInfo } from "@/types";

/**
 * Order apps for the launcher. With no query, recently opened apps lead; with a query, apps whose
 * name starts with it lead. Otherwise the incoming (alphabetical) order is kept.
 */
export function rankLauncherApps(apps: AppInfo[], query: string, recents: string[]): AppInfo[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return recentsFirst(apps, recents);

  const matches = filterApps(apps, needle, "all");
  const prefixed = matches.filter((app) => app.label.toLowerCase().startsWith(needle));
  const others = matches.filter((app) => !app.label.toLowerCase().startsWith(needle));
  return [...prefixed, ...others];
}

function recentsFirst(apps: AppInfo[], recents: string[]): AppInfo[] {
  const byPackage = new Map(apps.map((app) => [app.package, app]));
  const recent = recents.flatMap((pkg) => byPackage.get(pkg) ?? []);
  const recentPackages = new Set(recent.map((app) => app.package));
  return [...recent, ...apps.filter((app) => !recentPackages.has(app.package))];
}
