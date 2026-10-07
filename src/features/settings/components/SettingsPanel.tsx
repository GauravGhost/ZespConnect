import { open } from "@tauri-apps/plugin-dialog";
import { RefreshCw, Search, Terminal } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useInstallLog } from "@/hooks/useInstallLog";
import { useCheckUpdates, useInstallTool, useSetToolPath, useToolsStatus } from "@/hooks/useTools";
import { errorMessage } from "@/lib/ipc";
import type { ToolKind } from "@/types";
import { ToolCard } from "./ToolCard";

export function SettingsPanel() {
  const { data: tools, refetch, isFetching } = useToolsStatus();
  const setPath = useSetToolPath();
  const install = useInstallTool();
  const checkUpdates = useCheckUpdates();
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

  function checkForUpdates() {
    checkUpdates.mutate(undefined, {
      onError: (error) => toast.error(errorMessage(error)),
      onSuccess: (status) => {
        const problems = [status.adb_update.error, status.scrcpy_update.error].filter(Boolean);
        if (problems.length > 0) toast.warning(`Update check incomplete — ${problems[0]}`);
        else toast.success("Checked for updates");
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
        <div className="flex shrink-0 items-center gap-1">
          <Button
            variant="outline"
            size="sm"
            className="gap-2"
            onClick={checkForUpdates}
            disabled={checkUpdates.isPending}
          >
            <Search className={checkUpdates.isPending ? "size-4 animate-pulse" : "size-4"} />
            Check for updates
          </Button>
          <Button variant="ghost" size="sm" className="gap-2" onClick={() => refetch()}>
            <RefreshCw className={isFetching ? "size-4 animate-spin" : "size-4"} />
            Re-check
          </Button>
        </div>
      </div>

      <div className="grid min-w-0 gap-4">
        <ToolCard
          kind="adb"
          title="ADB (Android Debug Bridge)"
          description="Required. Talks to your Android devices."
          info={tools?.adb}
          update={tools?.adb_update}
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
          update={tools?.scrcpy_update}
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
