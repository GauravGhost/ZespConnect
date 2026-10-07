import { useVirtualizer } from "@tanstack/react-virtual";
import { open } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, Loader2, Package, RefreshCw, Search, Upload } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { EmptyState } from "@/components/app/EmptyState";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useActiveDevice } from "@/features/devices";
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
import {
  useAppList,
  useClearAppData,
  useInstallApk,
  useLaunchApp,
  useRefreshAppList,
  useUninstallApp,
} from "../hooks";
import { AppConfirmDialog, type AppConfirmAction } from "./AppConfirmDialog";
import { AppRow } from "./AppRow";

const FILTERS: { id: AppFilter; label: string }[] = [
  { id: "user", label: "User" },
  { id: "system", label: "System" },
  { id: "all", label: "All" },
];

const ROW_HEIGHT = 56;

export function AppsPanel() {
  const { device } = useActiveDevice();
  const serial = device?.serial ?? null;

  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<AppFilter>("user");
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

  const scrollRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: apps.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
  });

  if (!device) {
    return (
      <div className="flex h-full items-center justify-center p-6">
        <EmptyState
          icon={Package}
          title="No device connected"
          description="Connect an Android device to browse and manage its installed apps."
        />
      </div>
    );
  }

  async function pickApk() {
    if (!serial) return;
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Android package", extensions: ["apk", "apks", "xapk"] }],
      title: "Select an APK to install",
    });
    if (typeof selected === "string") installApk.mutate({ serial, path: selected });
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

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="shrink-0 space-y-3 border-b p-6 pb-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="min-w-0">
            <h1 className="text-lg font-semibold">Apps</h1>
            <p className="text-muted-foreground truncate text-sm">
              {device.display_name || device.serial} · {apps.length} shown
            </p>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            <Button
              variant="ghost"
              size="sm"
              className="gap-2"
              disabled={refreshing}
              onClick={() => serial && refreshApps({ serial, includeSystem })}
            >
              <RefreshCw className={cn("size-4", refreshing && "animate-spin")} />
              Refresh
            </Button>
            <Button size="sm" className="gap-2" disabled={installApk.isPending} onClick={pickApk}>
              {installApk.isPending ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Upload className="size-4" />
              )}
              Install APK
            </Button>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <div className="relative min-w-0 flex-1">
            <Search className="text-muted-foreground pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2" />
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search name or package"
              className="pl-8"
            />
          </div>
          <div className="flex items-center gap-1">
            {FILTERS.map((option) => (
              <Button
                key={option.id}
                size="sm"
                variant={filter === option.id ? "secondary" : "ghost"}
                onClick={() => setFilter(option.id)}
              >
                {option.label}
              </Button>
            ))}
          </div>
        </div>

        {data?.stale ? (
          <p className="text-muted-foreground text-xs">Showing cached names — refreshing…</p>
        ) : null}
      </div>

      <div ref={scrollRef} className="min-h-0 flex-1 overflow-auto px-6">
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
            {virtualizer.getVirtualItems().map((item) => {
              const app = apps[item.index];
              return (
                <div
                  key={app.package}
                  className="absolute top-0 left-0 w-full border-b"
                  style={{ height: item.size, transform: `translateY(${item.start}px)` }}
                >
                  <AppRow
                    app={app}
                    busy={rowBusy}
                    isOpen={windowFor(app.package).length > 0}
                    onOpen={(target) =>
                      serial &&
                      openWindow.mutate({ serial, pkg: target.package, label: target.label })
                    }
                    onOpenScreen={(target) =>
                      serial &&
                      openScreen.mutate({ serial, pkg: target.package, label: target.label })
                    }
                    onStop={(target) =>
                      windowFor(target.package).forEach((window) => stopWindow.mutate(window))
                    }
                    onLaunch={(target) => serial && launch.mutate({ serial, pkg: target.package })}
                    onClearData={(target) => setConfirm({ app: target, action: "clearData" })}
                    onUninstall={(target) => setConfirm({ app: target, action: "uninstall" })}
                  />
                </div>
              );
            })}
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
