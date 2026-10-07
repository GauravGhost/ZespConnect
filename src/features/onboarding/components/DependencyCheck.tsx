import { open } from "@tauri-apps/plugin-dialog";
import { Check, Copy, FolderSearch, RefreshCw, Terminal } from "lucide-react";
import { useState } from "react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { toast } from "sonner";

import { useSetToolPath, useToolsStatus } from "../hooks";

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

const INSTALL: Record<Os, Record<"adb" | "scrcpy", Command[]>> = {
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
    <div className="flex items-center justify-between gap-2 rounded-md border bg-muted/40 px-3 py-2">
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
  name,
  tool,
  os,
  onBrowse,
}: {
  name: "adb" | "scrcpy";
  tool: { found: boolean; path: string | null; version: string | null; source: string };
  os: Os;
  onBrowse: () => void;
}) {
  const label = name === "adb" ? "ADB (Android Debug Bridge)" : "scrcpy (Screen Copy)";
  return (
    <div className="space-y-3 rounded-lg border p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Terminal className="size-4" />
          <span className="text-sm font-medium">{label}</span>
        </div>
        {tool.found ? (
          <Badge variant="success">
            <Check /> Found
          </Badge>
        ) : (
          <Badge variant="warning">Not found</Badge>
        )}
      </div>

      {tool.found ? (
        <p className="text-muted-foreground truncate font-mono text-xs">
          {tool.path}
          {tool.version ? ` · ${tool.version}` : ""}
        </p>
      ) : (
        <>
          <p className="text-muted-foreground text-sm">
            Install {name} with your package manager, or point Zesp Connect at an existing binary.
          </p>
          <div className="grid gap-2">
            {INSTALL[os][name].map((command) => (
              <CopyCommand key={command.command} command={command} />
            ))}
          </div>
          <Button variant="outline" size="sm" onClick={onBrowse} className="gap-2">
            <FolderSearch className="size-4" /> Locate {name} binary
          </Button>
        </>
      )}
    </div>
  );
}

export function DependencyCheck() {
  const { data: tools, isLoading, isFetching, refetch } = useToolsStatus();
  const setToolPath = useSetToolPath();
  const os = detectOs();

  async function browse(name: "adb" | "scrcpy") {
    const selected = await open({
      multiple: false,
      directory: false,
      title: `Select the ${name} executable`,
    });
    if (typeof selected === "string") {
      setToolPath.mutate({ tool: name, path: selected });
    }
  }

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
          <ToolRow name="adb" tool={tools.adb} os={os} onBrowse={() => browse("adb")} />
          <ToolRow name="scrcpy" tool={tools.scrcpy} os={os} onBrowse={() => browse("scrcpy")} />
        </>
      )}

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
