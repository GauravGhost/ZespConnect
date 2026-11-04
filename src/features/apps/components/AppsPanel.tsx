import { useVirtualizer } from "@tanstack/react-virtual";
import { AlertTriangle, Loader2, Package, RefreshCw, Search, Upload } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { useAppStore } from "@/app/store";
import { EmptyState, NoDeviceState } from "@/components/app/EmptyState";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useActiveDevice } from "@/hooks/useDevices";
import { useElementWidth } from "@/hooks/useElementWidth";
import {
  useOpenAppOnScreen,
  useOpenAppWindow,
  useOpenWindows,
  useStopOpenWindow,
} from "@/hooks/useMirrors";
import { errorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { AppInfo } from "@/types";
import { filterApps, type AppFilter } from "../filter";
import { useInstallApk } from "@/hooks/useInstallApk";
import {
  useAppList,
  useClearAppData,
  useLaunchApp,
  useRefreshAppList,
  useUninstallApp,
} from "../hooks";
import type { AppActionHandlers } from "./AppActionsMenu";
import { AppConfirmDialog, type AppConfirmAction } from "./AppConfirmDialog";
import { AppRow } from "./AppRow";
import { AppsViewToggle } from "./AppsViewToggle";
import { AppTile } from "./AppTile";

const FILTERS: { id: AppFilter; label: string }[] = [
  { id: "user", label: "User" },
  { id: "system", label: "System" },
  { id: "all", label: "All" },
];

const ROW_HEIGHT = 56;
const MIN_TILE_WIDTH = 176;
const TILE_HEIGHT = 108;
const TILE_GAP = 12;

export function AppsPanel() {
  const { device } = useActiveDevice();
  const serial = device?.serial ?? null;

  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<AppFilter>("user");
  const view = useAppStore((state) => state.appsView);
  const setView = useAppStore((state) => state.setAppsView);
  const [confirm, setConfirm] = useState<{ app: AppInfo; action: AppConfirmAction } | null>(null);

  const includeSystem = filter !== "user";
  const { data, isLoading, isError, error } = useAppList(serial, includeSystem);

  const { mutate: refreshApps, isPending: refreshing } = useRefreshAppList();
  const launch = useLaunchApp();
  const installApk = useInstallApk();
  const clearData = useClearAppData();
  const uninstall = useUninstallApp();

  const { windows } = useOpenWindows();
  const openWindow = useOpenAppWindow();
  const openScreen = useOpenAppOnScreen();
  const stopWindow = useStopOpenWindow();
  const windowFor = (pkg: string) => windows.filter((window) => window.package === pkg);

  // When we painted from cache, pull the real list from the device in the background.
  const stale = Boolean(data?.stale);
  useEffect(() => {
    if (serial && stale) refreshApps({ serial, includeSystem });
  }, [serial, includeSystem, stale, refreshApps]);

  const apps = useMemo(
    () => (data ? filterApps(data.apps, query, filter) : []),
    [data, query, filter],
  );

  // Grid mode virtualizes whole rows of tiles, so the column count drives the row count.
  const [scrollElement, setScrollElement] = useState<HTMLDivElement | null>(null);
  const scrollWidth = useElementWidth(scrollElement);
  const columns =
    view === "grid"
      ? Math.max(1, Math.floor((scrollWidth + TILE_GAP) / (MIN_TILE_WIDTH + TILE_GAP)))
      : 1;
  const rowHeight = view === "grid" ? TILE_HEIGHT + TILE_GAP : ROW_HEIGHT;
  const virtualizer = useVirtualizer({
    count: Math.ceil(apps.length / columns),
    getScrollElement: () => scrollElement,
    estimateSize: () => rowHeight,
    paddingStart: view === "grid" ? TILE_GAP : 0,
    overscan: view === "grid" ? 3 : 10,
  });

  useEffect(() => {
    virtualizer.measure();
  }, [virtualizer, rowHeight]);

  if (!device) {
    return (
      <NoDeviceState
        icon={Package}
        description="Connect an Android device to browse and manage its installed apps."
      />
    );
  }

  function runConfirm() {
    if (!confirm || !serial) return;
    const { app, action } = confirm;
    const mutate = action === "uninstall" ? uninstall.mutate : clearData.mutate;
    mutate({ serial, pkg: app.package }, { onSettled: () => setConfirm(null) });
  }

  const confirmPending = uninstall.isPending || clearData.isPending;
  const rowBusy =
    launch.isPending ||
    confirmPending ||
    openWindow.isPending ||
    openScreen.isPending ||
    stopWindow.isPending;

  const handlers: AppActionHandlers = {
    onOpen: (target) =>
      serial && openWindow.mutate({ serial, pkg: target.package, label: target.label }),
    onOpenScreen: (target) =>
      serial && openScreen.mutate({ serial, pkg: target.package, label: target.label }),
    onStop: (target) => windowFor(target.package).forEach((window) => stopWindow.mutate(window)),
    onLaunch: (target) => serial && launch.mutate({ serial, pkg: target.package }),
    onClearData: (target) => setConfirm({ app: target, action: "clearData" }),
    onUninstall: (target) => setConfirm({ app: target, action: "uninstall" }),
  };

  function itemProps(app: AppInfo) {
    return { app, busy: rowBusy, isOpen: windowFor(app.package).length > 0, ...handlers };
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
              placeholder="Search name or package"
              className="h-9 pr-16 pl-9"
            />
            <span className="text-muted-foreground pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 text-xs tabular-nums">
              {apps.length}
            </span>
          </div>
          <div className="bg-muted flex shrink-0 items-center gap-0.5 rounded-full p-0.5">
            {FILTERS.map((option) => (
              <button
                key={option.id}
                type="button"
                aria-pressed={filter === option.id}
                onClick={() => setFilter(option.id)}
                className={cn(
                  "h-8 rounded-full px-3 text-xs font-medium transition-all duration-150 ease-emphasized",
                  filter === option.id
                    ? "bg-primary-container text-on-primary-container shadow-xs"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {option.label}
              </button>
            ))}
          </div>
          <AppsViewToggle value={view} onChange={setView} />
          <Button
            variant="ghost"
            size="icon"
            className="size-9 shrink-0"
            aria-label="Refresh app list"
            title="Refresh app list"
            disabled={refreshing}
            onClick={() => serial && refreshApps({ serial, includeSystem })}
          >
            <RefreshCw className={cn("size-4", refreshing && "animate-spin")} />
          </Button>
          <Button
            size="sm"
            className="h-9 shrink-0 gap-2"
            disabled={installApk.isPending}
            onClick={() => serial && void installApk.pickAndInstall(serial)}
          >
            {installApk.isPending ? (
              <Loader2 className="size-4 animate-spin" />
            ) : (
              <Upload className="size-4" />
            )}
            Install APK
          </Button>
        </div>

        {data?.stale ? (
          <p className="text-muted-foreground text-xs">Showing cached names - refreshing…</p>
        ) : null}
      </div>

      <div ref={setScrollElement} className="min-h-0 flex-1 overflow-auto px-3">
        {isLoading ? (
          <div className="flex h-40 items-center justify-center">
            <Loader2 className="text-muted-foreground size-5 animate-spin" />
          </div>
        ) : isError ? (
          <Alert variant="destructive" className="my-6">
            <AlertTriangle />
            <AlertTitle>Could not load apps</AlertTitle>
            <AlertDescription>{errorMessage(error)}</AlertDescription>
          </Alert>
        ) : apps.length === 0 ? (
          <div className="py-10">
            <EmptyState
              icon={Package}
              title="No apps found"
              description={
                query.trim()
                  ? "No app matches your search. Try a different name or package."
                  : "Nothing here for this filter. Switch to User, System or All."
              }
            />
          </div>
        ) : (
          <div style={{ height: virtualizer.getTotalSize(), position: "relative", width: "100%" }}>
            {virtualizer.getVirtualItems().map((item) =>
              view === "grid" ? (
                <div
                  key={item.key}
                  className="absolute top-0 left-0 grid w-full"
                  style={{
                    height: TILE_HEIGHT,
                    gap: TILE_GAP,
                    gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                    transform: `translateY(${item.start}px)`,
                  }}
                >
                  {apps.slice(item.index * columns, (item.index + 1) * columns).map((app) => (
                    <AppTile key={app.package} {...itemProps(app)} />
                  ))}
                </div>
              ) : (
                <div
                  key={apps[item.index].package}
                  className="absolute top-0 left-0 w-full border-b"
                  style={{ height: item.size, transform: `translateY(${item.start}px)` }}
                >
                  <AppRow {...itemProps(apps[item.index])} />
                </div>
              ),
            )}
          </div>
        )}
      </div>

      <AppConfirmDialog
        app={confirm?.app ?? null}
        action={confirm?.action ?? "uninstall"}
        pending={confirmPending}
        onOpenChange={(open) => {
          if (!open) setConfirm(null);
        }}
        onConfirm={runConfirm}
      />
    </div>
  );
}
