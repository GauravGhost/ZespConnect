import { Check, Copy, DownloadCloud, FolderSearch, RefreshCw, Terminal } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { InstallerLogOutput } from "@/components/app/InstallerLogOutput";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useToolActions, useToolsStatus } from "@/hooks/useTools";
import type { ToolInfo, ToolKind, UpdateInfo } from "@/types";

type Os = "windows" | "macos" | "linux";

function detectOs(): Os {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("win")) return "windows";
  if (ua.includes("mac")) return "macos";
  return "linux";
}

interface Command {
  manager: string;
  command: string;
}

const INSTALL: Record<Os, Record<ToolKind, Command[]>> = {
  windows: {
    adb: [
      { manager: "winget", command: "winget install Google.PlatformTools" },
      { manager: "scoop", command: "scoop install adb" },
      { manager: "choco", command: "choco install adb" },
    ],
    scrcpy: [
      { manager: "winget", command: "winget install Genymobile.scrcpy" },
      { manager: "scoop", command: "scoop install scrcpy" },
      { manager: "choco", command: "choco install scrcpy" },
    ],
  },
  macos: {
    adb: [{ manager: "brew", command: "brew install --cask android-platform-tools" }],
    scrcpy: [{ manager: "brew", command: "brew install scrcpy" }],
  },
  linux: {
    adb: [
      { manager: "Debian/Ubuntu", command: "sudo apt install android-tools-adb" },
      { manager: "Fedora", command: "sudo dnf install android-tools" },
      { manager: "Arch", command: "sudo pacman -S android-tools" },
    ],
    scrcpy: [
      { manager: "Debian/Ubuntu", command: "sudo apt install scrcpy" },
      { manager: "Fedora", command: "sudo dnf install scrcpy" },
      { manager: "Arch", command: "sudo pacman -S scrcpy" },
    ],
  },
};

function CopyCommand({ command }: { command: Command }) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(command.command);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error("Could not copy to clipboard");
    }
  }

  return (
    <div className="bg-muted/40 flex items-center justify-between gap-2 rounded-md border px-3 py-2">
      <div className="min-w-0">
        <div className="text-muted-foreground text-xs">{command.manager}</div>
        <code className="block truncate font-mono text-xs">{command.command}</code>
      </div>
      <Button variant="ghost" size="icon" onClick={copy} aria-label="Copy command">
        {copied ? <Check className="text-success" /> : <Copy />}
      </Button>
    </div>
  );
}

function ToolRow({
  kind,
  label,
  tool,
  update,
  os,
  busy,
  onBrowse,
  onInstall,
}: {
  kind: ToolKind;
  label: string;
  tool: ToolInfo;
  update: UpdateInfo | undefined;
  os: Os;
  busy: boolean;
  onBrowse: (kind: ToolKind) => void;
  onInstall: (kind: ToolKind) => void;
}) {
  const outdated = Boolean(update?.outdated);
  const latest = update?.latest_version ?? null;

  return (
    <div className="space-y-3 rounded-lg border p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Terminal className="size-4" />
          <span className="text-sm font-medium">{label}</span>
        </div>
        {tool.found ? (
          <Badge variant={outdated ? "warning" : "success"}>
            <Check /> {outdated ? "Update available" : "Found"}
          </Badge>
        ) : (
          <Badge variant="warning">Not found</Badge>
        )}
      </div>

      {tool.found ? (
        <div className="space-y-2">
          <p className="text-muted-foreground truncate font-mono text-xs">
            {tool.path}
            {tool.version ? ` · ${tool.version}` : ""}
          </p>
          {outdated ? (
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-muted-foreground text-xs">
                {latest ? `v${latest} is available` : "A newer version is available"}
              </span>
              <Button size="sm" className="gap-2" onClick={() => onInstall(kind)} disabled={busy}>
                <DownloadCloud className="size-4" />
                {latest ? `Update to v${latest}` : "Update"}
              </Button>
            </div>
          ) : null}
        </div>
      ) : (
        <>
          <p className="text-muted-foreground text-sm">
            Install {kind} automatically, or with your package manager.
            {latest ? ` Latest is v${latest}.` : ""}
          </p>

          <div className="flex flex-wrap gap-2">
            <Button size="sm" className="gap-2" onClick={() => onInstall(kind)} disabled={busy}>
              <DownloadCloud className="size-4" />
              Install
            </Button>
            <Button variant="outline" size="sm" className="gap-2" onClick={() => onBrowse(kind)}>
              <FolderSearch className="size-4" /> Locate binary…
            </Button>
          </div>

          <details className="text-sm">
            <summary className="text-muted-foreground cursor-pointer">Or install manually</summary>
            <div className="mt-2 grid gap-2">
              {INSTALL[os][kind].map((command) => (
                <CopyCommand key={command.command} command={command} />
              ))}
            </div>
          </details>
        </>
      )}
    </div>
  );
}

export function DependencyCheck() {
  const { data: tools, isLoading, isFetching, refetch } = useToolsStatus();
  const { log, locate, install, installing } = useToolActions();
  const os = detectOs();

  return (
    <div className="mx-auto max-w-2xl space-y-4 p-6">
      <Alert>
        <Terminal />
        <AlertTitle>Set up your tools</AlertTitle>
        <AlertDescription>
          Zesp Connect talks to your device through adb, and mirrors it with scrcpy. Install them
          once and this screen disappears.
        </AlertDescription>
      </Alert>

      {isLoading || !tools ? (
        <div className="text-muted-foreground flex items-center gap-2 p-4 text-sm">
          <RefreshCw className="size-4 animate-spin" /> Checking for adb and scrcpy…
        </div>
      ) : (
        <>
          <ToolRow
            kind="adb"
            label="ADB (Android Debug Bridge)"
            tool={tools.adb}
            update={tools.adb_update}
            os={os}
            busy={installing}
            onBrowse={locate}
            onInstall={install}
          />
          <ToolRow
            kind="scrcpy"
            label="scrcpy (Screen Copy)"
            tool={tools.scrcpy}
            update={tools.scrcpy_update}
            os={os}
            busy={installing}
            onBrowse={locate}
            onInstall={install}
          />
        </>
      )}

      {log.running || log.lines.length > 0 ? (
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">
              Installer output{log.tool ? ` · ${log.tool}` : ""}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <InstallerLogOutput lines={log.lines} className="max-h-40" />
          </CardContent>
        </Card>
      ) : null}

      <div className="flex justify-end">
        <Button variant="ghost" size="sm" className="gap-2" onClick={() => refetch()}>
          <RefreshCw className={isFetching ? "animate-spin" : undefined} /> Re-check
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">Enable USB debugging on your phone</CardTitle>
          <CardDescription>
            Settings → About phone → tap Build number 7 times, then Developer options → USB
            debugging. Accept the prompt when you plug in.
          </CardDescription>
        </CardHeader>
        <CardContent className="text-muted-foreground text-xs">
          {os === "windows" ? "Windows" : os === "macos" ? "macOS" : "Linux"} detected.
        </CardContent>
      </Card>
    </div>
  );
}
