import { RefreshCw, Search, Terminal } from "lucide-react";
import { toast } from "sonner";

import { InstallerLogOutput } from "@/components/app/InstallerLogOutput";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useCheckUpdates, useToolActions, useToolsStatus } from "@/hooks/useTools";
import { errorMessage } from "@/lib/ipc";
import type { ToolKind } from "@/types";
import { AppearanceCard } from "./AppearanceCard";
import { ToolCard } from "./ToolCard";

export function SettingsPanel() {
  const { data: tools, refetch, isFetching } = useToolsStatus();
  const { log, locate, clear, install, installing } = useToolActions();
  const checkUpdates = useCheckUpdates();

  function runInstall(kind: ToolKind) {
    install(kind, (status) => {
      const found = kind === "adb" ? status.adb.found : status.scrcpy.found;
      if (found) toast.success(`${kind} is ready`);
      else toast.warning(`Installed, but ${kind} was not found - set its path manually`);
    });
  }

  function checkForUpdates() {
    checkUpdates.mutate(undefined, {
      onError: (error) => toast.error(errorMessage(error)),
      onSuccess: (status) => {
        const problems = [status.adb_update.error, status.scrcpy_update.error].filter(Boolean);
        if (problems.length > 0) toast.warning(`Update check incomplete - ${problems[0]}`);
        else toast.success("Checked for updates");
      },
    });
  }

  return (
    <div className="mx-auto w-full max-w-5xl space-y-4 p-4">
      <AppearanceCard />

      <div className="flex flex-wrap items-center justify-between gap-2 pt-1">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold">Tools</h2>
          <p className="text-muted-foreground text-xs">
            The adb and scrcpy binaries Zesp Connect uses.
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

      <div className="grid min-w-0 gap-4 md:grid-cols-2">
        <ToolCard
          kind="adb"
          title="ADB"
          description="Android Debug Bridge. Required to talk to your devices."
          info={tools?.adb}
          update={tools?.adb_update}
          busy={installing}
          onLocate={locate}
          onClear={clear}
          onInstall={runInstall}
        />
        <ToolCard
          kind="scrcpy"
          title="scrcpy"
          description="Screen Copy. Optional, needed for mirroring."
          info={tools?.scrcpy}
          update={tools?.scrcpy_update}
          busy={installing}
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
            <InstallerLogOutput lines={log.lines} className="max-h-52" />
          </CardContent>
        </Card>
      ) : null}
    </div>
  );
}
