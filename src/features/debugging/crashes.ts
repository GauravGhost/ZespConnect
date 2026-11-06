import type { CrashKind, CrashReport } from "@/types";

export const KIND_LABELS: Record<CrashKind, string> = {
  crash: "Crash",
  nativeCrash: "Native crash",
  anr: "ANR",
};

export interface AppCrashes {
  app: string;
  reports: CrashReport[];
}

/** Groups newest-first reports by app; the app with the latest report comes first. */
export function groupByApp(reports: CrashReport[]): AppCrashes[] {
  const groups = new Map<string, CrashReport[]>();
  for (const report of reports) {
    const group = groups.get(report.app);
    if (group) group.push(report);
    else groups.set(report.app, [report]);
  }
  return [...groups].map(([app, appReports]) => ({ app, reports: appReports }));
}

/** Stable across refreshes, so a selection survives one. */
export function reportKey(report: CrashReport): string {
  return `${report.source} ${report.time} ${report.app}`;
}
