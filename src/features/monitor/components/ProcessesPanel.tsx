import { useVirtualizer } from "@tanstack/react-virtual";
import {
  ArrowDown,
  ArrowUp,
  Cog,
  ListTree,
  Loader2,
  Package,
  Pause,
  Play,
  Search,
  Square,
} from "lucide-react";
import { useMemo, useState } from "react";

import { AppIcon } from "@/components/app/AppIcon";
import { EmptyState, NoDeviceState } from "@/components/app/EmptyState";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useAppList } from "@/features/apps";
import { useActiveDevice } from "@/hooks/useDevices";
import { cn } from "@/lib/utils";
import type { ProcessInfo } from "@/types";
import { formatBytes } from "../format";
import { useForceStop, useProcesses } from "../hooks";
import {
  nextSort,
  processTitle,
  visibleProcesses,
  type ProcessScope,
  type ProcessSort,
  type ProcessSortKey,
} from "../processes";

const ROW_HEIGHT = 52;
const COLUMNS = "grid-cols-[minmax(0,1fr)_4.5rem_7.5rem_5.5rem_2.5rem]";
const KB = 1024;

const SCOPES: { id: ProcessScope; label: string }[] = [
  { id: "apps", label: "Apps" },
  { id: "all", label: "All" },
];

const STATE_LABELS: Record<string, string> = {
  R: "Running",
  S: "Sleeping",
  D: "Waiting on I/O",
  I: "Idle",
  T: "Stopped",
  Z: "Zombie",
};

function SortHeader({
  label,
  column,
  sort,
  onSort,
  className,
  title,
}: {
  label: string;
  column: ProcessSortKey;
  sort: ProcessSort;
  onSort: (key: ProcessSortKey) => void;
  className?: string;
  title?: string;
}) {
  const active = sort.key === column;
  const Arrow = sort.descending ? ArrowDown : ArrowUp;
  return (
    <button
      type="button"
      title={title}
      onClick={() => onSort(column)}
      aria-sort={active ? (sort.descending ? "descending" : "ascending") : "none"}
      className={cn(
        "hover:text-foreground inline-flex items-center gap-1 text-xs font-semibold tracking-wide uppercase transition-colors",
        active ? "text-foreground" : "text-muted-foreground",
        className,
      )}
    >
      {label}
      {active ? <Arrow className="size-3" /> : null}
    </button>
  );
}

/** Usage against one core; a process spread across cores can pass 100%. */
function CpuCell({ cpu }: { cpu: number }) {
  return (
    <div className="flex items-center gap-2">
      <div className="bg-muted/70 h-1.5 flex-1 overflow-hidden rounded-full">
        <div
          className="bg-primary h-full rounded-full transition-[width] duration-500 ease-standard"
          style={{ width: `${Math.min(100, cpu)}%` }}
        />
      </div>
      <span className="w-10 text-right text-xs tabular-nums">{cpu.toFixed(1)}%</span>
    </div>
  );
}

function ProcessRow({
  process,
  iconPath,
  onStop,
}: {
  process: ProcessInfo;
  iconPath: string | null;
  onStop: (process: ProcessInfo) => void;
}) {
  const FallbackIcon = process.package ? Package : Cog;
  return (
    <div className={cn("grid h-full items-center gap-3 px-4", COLUMNS)}>
      <div className="flex min-w-0 items-center gap-3">
        <AppIcon
          iconPath={iconPath}
          className="size-8 rounded-xl"
          fallback={
            <span className="bg-muted/70 text-muted-foreground flex size-8 shrink-0 items-center justify-center rounded-xl">
              <FallbackIcon className="size-4" />
            </span>
          }
        />
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{processTitle(process)}</p>
          <p className="text-muted-foreground truncate text-xs">
            {process.label ? `${process.name} · ` : ""}
            {process.user} · {STATE_LABELS[process.state] ?? process.state}
          </p>
        </div>
      </div>
      <span className="text-muted-foreground text-right text-xs tabular-nums">{process.pid}</span>
      <CpuCell cpu={process.cpu} />
      <span className="text-right text-xs tabular-nums">{formatBytes(process.resKb * KB)}</span>
      {process.package ? (
        <Button
          variant="ghost"
          size="icon"
          className="hover:text-destructive size-8 justify-self-end"
          aria-label={`Force stop ${processTitle(process)}`}
          title="Force stop"
          onClick={() => onStop(process)}
        >
          <Square className="size-3.5" />
        </Button>
      ) : (
        <span />
      )}
    </div>
  );
}

function ForceStopDialog({
  target,
  pending,
  onOpenChange,
  onConfirm,
}: {
  target: ProcessInfo | null;
  pending: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}) {
  return (
    <Dialog open={target !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Force stop app?</DialogTitle>
          <DialogDescription>
            {target
              ? `Every process of ${processTitle(target)} stops now. Unsaved work in the app may be lost.`
              : ""}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={onConfirm} disabled={pending}>
            {pending ? <Loader2 className="size-4 animate-spin" /> : null}
            Force stop
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function ProcessesPanel() {
  const { device } = useActiveDevice();
  const serial = device?.state === "device" ? device.serial : null;
  const { processes, paused, setPaused, error } = useProcesses(serial);
  const { data: appList } = useAppList(serial, false);
  const forceStop = useForceStop(serial);

  const [query, setQuery] = useState("");
  const [scope, setScope] = useState<ProcessScope>("apps");
  const [sort, setSort] = useState<ProcessSort>({ key: "cpu", descending: true });
  const [target, setTarget] = useState<ProcessInfo | null>(null);
  const [scrollElement, setScrollElement] = useState<HTMLDivElement | null>(null);

  const icons = useMemo(
    () => new Map((appList?.apps ?? []).map((app) => [app.package, app.icon_path])),
    [appList],
  );
  const rows = useMemo(
    () => visibleProcesses(processes ?? [], query, scope, sort),
    [processes, query, scope, sort],
  );
  const appCount = useMemo(
    () => new Set((processes ?? []).flatMap((p) => (p.package ? [p.package] : []))).size,
    [processes],
  );

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollElement,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
  });

  if (!serial) {
    return (
      <NoDeviceState
        icon={ListTree}
        description="Connect an Android device to see what is running on it."
      />
    );
  }

  const onSort = (key: ProcessSortKey) => setSort((current) => nextSort(current, key));

  function confirmStop() {
    const pkg = target?.package;
    if (!pkg) return;
    forceStop.mutate(pkg, { onSettled: () => setTarget(null) });
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="shrink-0 space-y-2 border-b px-4 py-3">
        <div className="flex items-center gap-2">
          <div className="relative min-w-0 flex-1">
            <Search className="text-muted-foreground pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2" />
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search name, command or PID"
              className="h-9 pr-16 pl-9"
            />
            <span className="text-muted-foreground pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 text-xs tabular-nums">
              {rows.length}
            </span>
          </div>
          <div className="bg-muted flex shrink-0 items-center gap-0.5 rounded-full p-0.5">
            {SCOPES.map((option) => (
              <button
                key={option.id}
                type="button"
                aria-pressed={scope === option.id}
                onClick={() => setScope(option.id)}
                className={cn(
                  "h-8 rounded-full px-3 text-xs font-medium transition-all duration-150 ease-emphasized",
                  scope === option.id
                    ? "bg-primary-container text-on-primary-container shadow-xs"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {option.label}
              </button>
            ))}
          </div>
          <Button
            variant="outline"
            size="sm"
            className="h-9 shrink-0"
            onClick={() => setPaused(!paused)}
          >
            {paused ? <Play /> : <Pause />}
            {paused ? "Resume" : "Pause"}
          </Button>
        </div>
        {processes ? (
          <p className="text-muted-foreground text-xs">
            {processes.length} processes · {appCount} apps running
            {paused ? " · paused" : " · refreshes every 3 seconds"}
          </p>
        ) : null}
      </div>

      {error ? (
        <div className="p-4">
          <Alert variant="destructive">
            <AlertTitle>Could not read the process list</AlertTitle>
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        </div>
      ) : processes === null ? (
        <div className="flex flex-1 items-center justify-center p-6">
          <EmptyState icon={Loader2} title="Reading processes…" />
        </div>
      ) : (
        <>
          <div className={cn("grid shrink-0 gap-3 border-b px-4 py-2", COLUMNS)}>
            <SortHeader label="Name" column="name" sort={sort} onSort={onSort} />
            <SortHeader
              label="PID"
              column="pid"
              sort={sort}
              onSort={onSort}
              className="justify-self-end"
            />
            <SortHeader
              label="CPU"
              column="cpu"
              sort={sort}
              onSort={onSort}
              title="Share of one core"
              className="justify-self-end"
            />
            <SortHeader
              label="Memory"
              column="memory"
              sort={sort}
              onSort={onSort}
              className="justify-self-end"
            />
            <span />
          </div>
          <div ref={setScrollElement} className="min-h-0 flex-1 overflow-y-auto">
            {rows.length === 0 ? (
              <EmptyState icon={Search} title="No matching processes" className="m-4" />
            ) : (
              <div className="relative" style={{ height: virtualizer.getTotalSize() }}>
                {virtualizer.getVirtualItems().map((item) => {
                  const process = rows[item.index];
                  return (
                    <div
                      key={process.pid}
                      className="hover:bg-muted/40 absolute inset-x-0 border-b border-border/40"
                      style={{ top: item.start, height: item.size }}
                    >
                      <ProcessRow
                        process={process}
                        iconPath={process.package ? (icons.get(process.package) ?? null) : null}
                        onStop={setTarget}
                      />
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </>
      )}

      <ForceStopDialog
        target={target}
        pending={forceStop.isPending}
        onOpenChange={(open) => !open && setTarget(null)}
        onConfirm={confirmStop}
      />
    </div>
  );
}
