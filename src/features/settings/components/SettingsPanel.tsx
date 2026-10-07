import { open } from "@tauri-apps/plugin-dialog";
import { RefreshCw, Terminal } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useInstallLog } from "@/hooks/useInstallLog";
import { useInstallTool, useSetToolPath, useToolsStatus } from "@/hooks/useTools";
import { errorMessage } from "@/lib/ipc";
import type { ToolKind } from "@/types";
import { ToolCard } from "./ToolCard";

export function SettingsPanel() {
  const { data: tools, refetch, isFetching } = useToolsStatus();
  const setPath = useSetToolPath();
  const install = useInstallTool();
  const log = useInstallLog();

  async function locate(kind: ToolKind) {
    const selected = await open({
      multiple: false,
      directory: false,
      title: `Select the ${kind} executable`,
    });
    if (typeof selected === "string") {
      setPath.mutate({ tool: kind, path: selected });
    }
  }

  function clear(kind: ToolKind) {
    setPath.mutate({ tool: kind, path: null });
  }

  function runInstall(kind: ToolKind) {
    log.reset();
    install.mutate(kind, {
      onError: (error) => toast.error(errorMessage(error)),
      onSuccess: (status) => {
        const found = kind === "adb" ? status.adb.found : status.scrcpy.found;
        if (found) toast.success(`${kind} is ready`);
        else toast.warning(`Installed, but ${kind} was not found — set its path manually`);
      },
    });
  }

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6 p-6">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <h1 className="text-lg font-semibold">Settings</h1>
          <p className="text-muted-foreground text-sm">
            Manage the adb and scrcpy binaries Zesp Connect uses.
          </p>
        </div>
        <Button variant="ghost" size="sm" className="shrink-0 gap-2" onClick={() => refetch()}>
          <RefreshCw className={isFetching ? "size-4 animate-spin" : "size-4"} />
          Re-check
        </Button>
      </div>

      <div className="grid min-w-0 gap-4">
        <ToolCard
          kind="adb"
          title="ADB (Android Debug Bridge)"
          description="Required. Talks to your Android devices."
          info={tools?.adb}
          busy={install.isPending}
          onLocate={locate}
          onClear={clear}
          onInstall={runInstall}
        />
        <ToolCard
          kind="scrcpy"
          title="scrcpy (Screen Copy)"
          description="Optional. Needed for screen and camera mirroring."
          info={tools?.scrcpy}
          busy={install.isPending}
          onLocate={locate}
          onClear={clear}
          onInstall={runInstall}
        />
      </div>

      {log.running || log.lines.length > 0 || log.message ? (
        <Card>
          <CardHeader className="flex-row items-center justify-between">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Terminal className="size-4" />
              Installer output{log.tool ? ` · ${log.tool}` : ""}
            </CardTitle>
            <Button variant="ghost" size="sm" onClick={log.reset}>
              Clear
            </Button>
          </CardHeader>
          <CardContent className="space-y-2">
            {log.message ? (
              <p className={log.ok ? "text-success text-sm" : "text-destructive text-sm"}>
                {log.message}
              </p>
            ) : null}
            <pre className="bg-muted/50 max-h-52 overflow-auto rounded-md p-3 font-mono text-xs whitespace-pre-wrap">
              {log.lines.length > 0 ? log.lines.join("\n") : "Waiting for output…"}
            </pre>
          </CardContent>
        </Card>
      ) : null}
    </div>
  );
}
