import { save } from "@tauri-apps/plugin-dialog";
import { Bug, Copy, Download, FileArchive, Loader2, RefreshCw } from "lucide-react";
import { useMemo, useState } from "react";
import { toast } from "sonner";

import { Elapsed } from "@/components/app/Elapsed";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { fileStamp } from "@/lib/format";
import { errorMessage, ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { CrashKind, CrashReport } from "@/types";
import { copyText } from "../copy";
import { groupByApp, KIND_LABELS, reportKey } from "../crashes";
import { useBugReportStart, useCrashReports, useSaveBugReport } from "../hooks";

const KIND_BADGES: Record<CrashKind, "destructive" | "warning"> = {
  crash: "destructive",
  nativeCrash: "destructive",
  anr: "warning",
};

function KindBadge({ kind }: { kind: CrashKind }) {
  return (
    <Badge variant={KIND_BADGES[kind]} className="shrink-0 text-[10px]">
      {KIND_LABELS[kind]}
    </Badge>
  );
}

function BugReportButton({ serial }: { serial: string }) {
  const capture = useSaveBugReport(serial);
  const startedAt = useBugReportStart(serial);

  async function start() {
    const path = await save({
      defaultPath: `bugreport-${fileStamp(new Date())}.zip`,
      filters: [{ name: "Zip", extensions: ["zip"] }],
      title: "Save bug report",
    });
    if (typeof path === "string") capture.mutate(path);
  }

  if (startedAt !== null) {
    return (
      <span
        className="text-muted-foreground flex items-center gap-2 text-xs"
        title="adb reports no progress; a bug report usually takes one to five minutes."
      >
        <Loader2 className="size-3.5 animate-spin" />
        Capturing bug report <Elapsed since={startedAt} />
      </span>
    );
  }
  return (
    <Button
      variant="outline"
      size="sm"
      onClick={() => void start()}
      title="Save a zip of the phone's logs, system state and traces, for a bug tracker"
    >
      <FileArchive />
      Bug report
    </Button>
  );
}

function ReportList({
  reports,
  selectedKey,
  onSelect,
}: {
  reports: CrashReport[];
  selectedKey: string | null;
  onSelect: (key: string) => void;
}) {
  const groups = useMemo(() => groupByApp(reports), [reports]);
  return (
    <div className="min-h-0 flex-1 overflow-auto">
      {groups.map((group) => (
        <section key={group.app}>
          <h3 className="bg-muted/60 text-muted-foreground sticky top-0 flex items-center gap-2 px-4 py-1.5 text-xs font-medium">
            <span className="min-w-0 flex-1 truncate font-mono">{group.app}</span>
            <span className="tabular-nums">{group.reports.length}</span>
          </h3>
          <ul>
            {group.reports.map((report) => {
              const key = reportKey(report);
              return (
                <li key={key}>
                  <button
                    type="button"
                    onClick={() => onSelect(key)}
                    className={cn(
                      "block w-full px-4 py-2 text-left",
                      key === selectedKey ? "bg-primary/15" : "hover:bg-accent",
                    )}
                  >
                    <span className="flex items-center gap-2">
                      <KindBadge kind={report.kind} />
                      <span className="text-muted-foreground text-xs tabular-nums">
                        {report.time}
                      </span>
                    </span>
                    <span className="mt-1 block truncate font-mono text-xs">{report.summary}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        </section>
      ))}
    </div>
  );
}

function ReportDetails({ report }: { report: CrashReport }) {
  async function saveReport() {
    const path = await save({
      defaultPath: `${report.app}-${report.kind}-${fileStamp(new Date())}.txt`,
      filters: [{ name: "Text", extensions: ["txt"] }],
      title: "Save report",
    });
    if (typeof path !== "string") return;
    try {
      await ipc.console.saveLog(path, report.details);
      toast.success("Report saved");
    } catch (failure) {
      toast.error(errorMessage(failure));
    }
  }

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      <div className="flex shrink-0 items-start gap-3 border-b px-4 py-3">
        <div className="min-w-0 flex-1 space-y-1">
          <p className="flex items-center gap-2">
            <KindBadge kind={report.kind} />
            <span className="truncate font-mono text-sm">{report.app}</span>
          </p>
          <p className="text-muted-foreground text-xs">
            {report.time} · {report.source}
          </p>
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => void copyText(report.details, "Report")}
          aria-label="Copy report"
          title="Copy report"
        >
          <Copy />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => void saveReport()}
          aria-label="Save report"
          title="Save report"
        >
          <Download />
        </Button>
      </div>
      <pre className="min-h-0 flex-1 overflow-auto p-4 font-mono text-xs leading-5 select-text">
        {report.details}
        {report.truncated ? (
          <span className="text-muted-foreground block pt-2">
            (Cut short. The full entry is in a bug report.)
          </span>
        ) : null}
      </pre>
    </div>
  );
}

export function CrashesView({ serial }: { serial: string }) {
  const query = useCrashReports(serial);
  const reports = useMemo(() => query.data ?? [], [query.data]);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  // Falls back to the newest report, also when a refresh drops the selected one.
  const selected =
    reports.find((report) => reportKey(report) === selectedKey) ?? reports[0] ?? null;

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 items-center gap-2 border-b px-4 py-2">
        <Button
          variant="outline"
          size="sm"
          onClick={() => void query.refetch()}
          disabled={query.isFetching}
        >
          <RefreshCw className={query.isFetching ? "animate-spin" : undefined} />
          Refresh
        </Button>
        <span className="text-muted-foreground text-xs">
          {reports.length === 1 ? "1 report" : `${reports.length} reports`}
        </span>
        <div className="ml-auto">
          <BugReportButton serial={serial} />
        </div>
      </div>

      {query.isError ? (
        <p className="text-destructive px-4 py-6 text-center text-sm">
          Could not read crash reports: {errorMessage(query.error)}
        </p>
      ) : selected ? (
        <div className="flex min-h-0 flex-1">
          <div className="flex w-[min(24rem,45%)] shrink-0 flex-col border-r">
            <ReportList
              reports={reports}
              selectedKey={reportKey(selected)}
              onSelect={setSelectedKey}
            />
          </div>
          <ReportDetails report={selected} />
        </div>
      ) : (
        <div className="text-muted-foreground flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center">
          {query.isPending ? (
            <Loader2 className="size-6 animate-spin" />
          ) : (
            <Bug className="size-6" />
          )}
          <p className="max-w-sm text-sm">
            {query.isPending
              ? "Reading crash reports."
              : "No crashes or ANRs recorded. The phone keeps them for about three days."}
          </p>
        </div>
      )}
    </div>
  );
}
