import { useQueryClient } from "@tanstack/react-query";
import { RefreshCw } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { navItem } from "@/app/navigation";
import { useAppStore } from "@/app/store";
import { ThemeModeToggle } from "@/components/app/ThemeModeToggle";
import { DeviceSwitcher } from "@/components/app/DeviceSwitcher";
import { QrPairDialog } from "@/components/app/QrPairDialog";
import { SIDEBAR_WIDTH } from "@/components/app/Sidebar";
import { WindowControls } from "@/features/mirror-viewer";
import { devicesKey, useActiveDevice } from "@/hooks/useDevices";
import { toolsKey } from "@/hooks/useTools";
import { IS_MAC } from "@/lib/platform";
import { cn } from "@/lib/utils";

/**
 * The window's title bar: the native one is hidden, so this bar drags the window. macOS keeps its
 * traffic lights on top of it; elsewhere it draws its own window controls.
 */
export function AppHeader() {
  const queryClient = useQueryClient();
  const activeTab = useAppStore((s) => s.activeTab);
  const { device, devices, setSelectedSerial } = useActiveDevice();
  const page = navItem(activeTab);

  return (
    <header
      data-tauri-drag-region
      className="bg-sidebar text-sidebar-foreground flex h-11 shrink-0 items-center gap-3 border-b border-sidebar-border pr-2 select-none"
    >
      <span
        data-tauri-drag-region
        className={cn(
          SIDEBAR_WIDTH,
          "shrink-0 font-mono text-sm font-medium tracking-tight",
          // Room for the traffic lights, placed by `trafficLightPosition` in tauri.conf.json.
          IS_MAC ? "pl-[84px]" : "pl-4",
        )}
      >
        zesp<span className="text-primary">/</span>connect
      </span>

      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-baseline gap-2">
        <h1 data-tauri-drag-region className="shrink-0 text-sm font-semibold tracking-tight">
          {page.label}
        </h1>
        <p data-tauri-drag-region className="text-muted-foreground truncate text-xs">
          {page.description}
        </p>
      </div>

      <div className="flex shrink-0 items-center gap-0.5">
        <QrPairDialog />
        <ThemeModeToggle />
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="size-8 rounded-lg"
              aria-label="Refresh"
              onClick={() => {
                queryClient.invalidateQueries({ queryKey: devicesKey });
                queryClient.invalidateQueries({ queryKey: toolsKey });
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

        {IS_MAC ? null : (
          <div className="ml-2 border-l border-sidebar-border pl-2">
            <WindowControls />
          </div>
        )}
      </div>
    </header>
  );
}
