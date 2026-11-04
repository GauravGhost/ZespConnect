import { RefreshCw, TriangleAlert, X } from "lucide-react";

import { useAppStore } from "@/app/store";
import { Button } from "@/components/ui/button";
import { useInstallLog } from "@/hooks/useInstallLog";
import { useToolsStatus } from "@/hooks/useTools";
import type { ToolStatus } from "@/types";

interface ScrcpyNotice {
  message: string;
  /** Set for update notices; dismissing hides the banner until a newer version appears. */
  updateVersion?: string;
}

const UNKNOWN_VERSION = "unknown";

function scrcpyNotice(tools: ToolStatus | undefined): ScrcpyNotice | null {
  if (!tools?.adb.found) return null;
  if (!tools.scrcpy.found) {
    return { message: "scrcpy is not installed. Mirroring and app windows need it." };
  }
  if (!tools.scrcpy_update.outdated) return null;

  const latest = tools.scrcpy_update.latest_version;
  const installed = tools.scrcpy.version;
  const available = latest ? `scrcpy v${latest} is available.` : "A scrcpy update is available.";
  return {
    message: installed ? `${available} You have v${installed}.` : available,
    updateVersion: latest ?? UNKNOWN_VERSION,
  };
}

/** Strip under the title bar for a running tool install or a missing / outdated scrcpy. */
export function ToolStatusBanner() {
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const dismissedScrcpyUpdate = useAppStore((s) => s.dismissedScrcpyUpdate);
  const dismissScrcpyUpdate = useAppStore((s) => s.dismissScrcpyUpdate);
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
  const { message, updateVersion } = notice;
  if (updateVersion && updateVersion === dismissedScrcpyUpdate) return null;

  return (
    <div className="bg-warning/10 text-warning border-warning/30 flex shrink-0 items-center gap-2 border-b px-4 py-1.5 text-xs font-medium">
      <TriangleAlert className="size-3.5 shrink-0" />
      <span className="min-w-0 flex-1 truncate">{message}</span>
      <Button
        variant="outline"
        size="sm"
        className="text-foreground h-7 rounded-lg text-xs"
        onClick={() => setActiveTab("settings")}
      >
        Open Settings
      </Button>
      {updateVersion && (
        <Button
          variant="ghost"
          size="icon"
          className="text-foreground size-7 rounded-lg"
          aria-label="Dismiss"
          title="Dismiss until the next update"
          onClick={() => dismissScrcpyUpdate(updateVersion)}
        >
          <X className="size-3.5" />
        </Button>
      )}
    </div>
  );
}
