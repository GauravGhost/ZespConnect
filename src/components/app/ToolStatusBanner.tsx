import { RefreshCw, TriangleAlert } from "lucide-react";

import { useAppStore } from "@/app/store";
import { Button } from "@/components/ui/button";
import { useInstallLog } from "@/hooks/useInstallLog";
import { useToolsStatus } from "@/hooks/useTools";
import type { ToolStatus } from "@/types";

function scrcpyNotice(tools: ToolStatus | undefined): string | null {
  if (!tools?.adb.found) return null;
  if (!tools.scrcpy.found) return "scrcpy is not installed. Mirroring and app windows need it.";
  if (!tools.scrcpy_update.outdated) return null;

  const latest = tools.scrcpy_update.latest_version;
  const installed = tools.scrcpy.version;
  const available = latest ? `scrcpy v${latest} is available.` : "A scrcpy update is available.";
  return installed ? `${available} You have v${installed}.` : available;
}

/** Strip under the title bar for a running tool install or a missing / outdated scrcpy. */
export function ToolStatusBanner() {
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const { data: tools } = useToolsStatus();
  const install = useInstallLog();

  if (install.running) {
    return (
      <div className="bg-muted/60 text-muted-foreground flex shrink-0 items-center gap-2 border-b border-border px-4 py-2 text-xs font-medium">
        <RefreshCw className="size-3.5 animate-spin" />
        Installing {install.tool ?? "tool"}…
      </div>
    );
  }

  const notice = scrcpyNotice(tools);
  if (!notice) return null;

  return (
    <div className="bg-warning/10 text-warning border-warning/30 flex shrink-0 items-center gap-2 border-b px-4 py-1.5 text-xs font-medium">
      <TriangleAlert className="size-3.5 shrink-0" />
      <span className="min-w-0 flex-1 truncate">{notice}</span>
      <Button
        variant="outline"
        size="sm"
        className="text-foreground h-7 rounded-lg text-xs"
        onClick={() => setActiveTab("settings")}
      >
        Open Settings
      </Button>
    </div>
  );
}
