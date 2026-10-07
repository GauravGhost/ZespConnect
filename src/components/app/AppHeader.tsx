import { useQueryClient } from "@tanstack/react-query";
import { RefreshCw, TriangleAlert } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { DeviceSwitcher } from "@/features/devices";
import { useActiveDevice } from "@/features/devices/hooks";
import { useToolsStatus } from "@/features/onboarding";

export function AppHeader() {
  const queryClient = useQueryClient();
  const { device, devices, setSelectedSerial } = useActiveDevice();
  const { data: tools } = useToolsStatus();

  const scrcpyMissing = Boolean(tools && tools.adb.found && !tools.scrcpy.found);

  return (
    <header className="flex h-14 shrink-0 items-center justify-between gap-4 border-b px-4">
      <div className="flex items-center gap-2">
        {scrcpyMissing ? (
          <Tooltip>
            <TooltipTrigger asChild>
              <span className="text-warning inline-flex items-center gap-1 text-xs font-medium">
                <TriangleAlert className="size-4" /> scrcpy missing — mirroring unavailable
              </span>
            </TooltipTrigger>
            <TooltipContent>
              Install scrcpy from the Overview tab to enable mirroring.
            </TooltipContent>
          </Tooltip>
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
