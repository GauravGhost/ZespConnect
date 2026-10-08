import { useQueryClient } from "@tanstack/react-query";
import { RefreshCw, TriangleAlert } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { navItem } from "@/app/navigation";
import { useAppStore } from "@/app/store";
import { ThemeModeToggle } from "@/components/app/ThemeModeToggle";
import { DeviceSwitcher } from "@/features/devices";
import { useActiveDevice } from "@/features/devices/hooks";
import { useInstallLog } from "@/hooks/useInstallLog";
import { useToolsStatus } from "@/hooks/useTools";

export function AppHeader() {
  const queryClient = useQueryClient();
  const activeTab = useAppStore((s) => s.activeTab);
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const { device, devices, setSelectedSerial } = useActiveDevice();
  const { data: tools } = useToolsStatus();
  const install = useInstallLog();
  const page = navItem(activeTab);

  const scrcpyMissing = Boolean(tools && tools.adb.found && !tools.scrcpy.found);
  const scrcpyOutdated = Boolean(tools?.scrcpy.found && tools.scrcpy_update.outdated);
  const scrcpyLatest = tools?.scrcpy_update.latest_version ?? null;

  return (
    <header className="bg-background/80 flex h-12 shrink-0 items-center justify-between gap-3 border-b border-border/60 pr-2 pl-4 backdrop-blur">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex min-w-0 items-baseline gap-2">
          <h1 className="shrink-0 text-[15px] font-semibold tracking-tight">{page.label}</h1>
          <p className="text-muted-foreground hidden truncate text-xs md:block">
            {page.description}
          </p>
        </div>

        {install.running ? (
          <span className="bg-muted text-muted-foreground inline-flex shrink-0 items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium">
            <RefreshCw className="size-3.5 animate-spin" />
            Installing {install.tool ?? "tool"}…
          </span>
        ) : scrcpyMissing || scrcpyOutdated ? (
          <button
            type="button"
            onClick={() => setActiveTab("settings")}
            className="bg-warning/15 text-warning hover:bg-warning/25 inline-flex shrink-0 items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium transition-colors"
          >
            <TriangleAlert className="size-3.5" />
            {scrcpyMissing
              ? "scrcpy missing"
              : `scrcpy ${scrcpyLatest ? `v${scrcpyLatest}` : "update"} available`}
          </button>
        ) : null}
      </div>

      <div className="flex shrink-0 items-center gap-0.5">
        <ThemeModeToggle />
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="size-9"
              aria-label="Refresh"
              onClick={() => {
                queryClient.invalidateQueries({ queryKey: ["devices"] });
                queryClient.invalidateQueries({ queryKey: ["tools"] });
              }}
            >
              <RefreshCw />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Refresh devices and tools</TooltipContent>
        </Tooltip>

        <div className="ml-1.5">
          <DeviceSwitcher device={device} devices={devices} onSelect={setSelectedSerial} />
        </div>
      </div>
    </header>
  );
}
