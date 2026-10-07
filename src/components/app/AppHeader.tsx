import { useQueryClient } from "@tanstack/react-query";
import { RefreshCw, TriangleAlert } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useAppStore } from "@/app/store";
import { DeviceSwitcher } from "@/features/devices";
import { useActiveDevice } from "@/features/devices/hooks";
import { useInstallLog } from "@/hooks/useInstallLog";
import { useToolsStatus } from "@/hooks/useTools";

export function AppHeader() {
  const queryClient = useQueryClient();
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const { device, devices, setSelectedSerial } = useActiveDevice();
  const { data: tools } = useToolsStatus();
  const install = useInstallLog();

  const scrcpyMissing = Boolean(tools && tools.adb.found && !tools.scrcpy.found);

  return (
    <header className="flex h-14 shrink-0 items-center justify-between gap-4 border-b px-4">
      <div className="flex items-center gap-2">
        {install.running ? (
          <span className="text-muted-foreground inline-flex items-center gap-1.5 text-xs font-medium">
            <RefreshCw className="size-3.5 animate-spin" />
            Installing {install.tool ?? "tool"}…
          </span>
        ) : scrcpyMissing ? (
          <button
            type="button"
            onClick={() => setActiveTab("settings")}
            className="text-warning hover:text-warning/80 inline-flex items-center gap-1 text-xs font-medium"
          >
            <TriangleAlert className="size-4" />
            scrcpy missing — set it up
          </button>
        ) : null}
      </div>

      <div className="flex items-center gap-2">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
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

        <DeviceSwitcher device={device} devices={devices} onSelect={setSelectedSerial} />
      </div>
    </header>
  );
}
