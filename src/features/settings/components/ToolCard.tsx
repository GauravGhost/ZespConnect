import { ChevronDown, DownloadCloud, Terminal } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { ToolInfo, ToolKind, UpdateInfo } from "@/types";

type SelectedSource = "auto" | "managed" | "custom";

const SELECTED_SOURCE_LABEL: Record<SelectedSource, string> = {
  auto: "Auto-detected",
  managed: "App-installed",
  custom: "Custom file",
};

function selectedSource(info: ToolInfo): SelectedSource {
  if (info.managed_path !== null && info.managed_path === info.path) return "managed";
  return info.source === "override" ? "custom" : "auto";
}

function sourceTriggerLabel(info: ToolInfo | undefined): string {
  if (!info) return "Checking…";
  if (!info.found) return "Not found";
  return SELECTED_SOURCE_LABEL[selectedSource(info)];
}

export function ToolCard({
  kind,
  title,
  description,
  info,
  update,
  busy,
  onLocate,
  onClear,
  onUseManaged,
  onInstall,
}: {
  kind: ToolKind;
  title: string;
  description: string;
  info: ToolInfo | undefined;
  update: UpdateInfo | undefined;
  busy: boolean;
  onLocate: (kind: ToolKind) => void;
  onClear: (kind: ToolKind) => void;
  onUseManaged: (kind: ToolKind, path: string) => void;
  onInstall: (kind: ToolKind) => void;
}) {
  const found = Boolean(info?.found);
  const selected = info?.found ? selectedSource(info) : null;
  const managedPath = info?.managed_path ?? null;
  const latest = update?.latest_version ?? null;
  const upToDate = found && latest !== null && !update?.outdated;

  return (
    <Card className="min-w-0 gap-3 py-4">
      <CardHeader className="flex-row items-start justify-between gap-3 px-4">
        <div className="min-w-0 space-y-1">
          <CardTitle className="flex min-w-0 items-center gap-2 text-sm">
            <Terminal className="size-4 shrink-0" />
            <span className="truncate">{title}</span>
            {found && info?.version ? (
              <span className="text-muted-foreground text-xs font-normal">v{info.version}</span>
            ) : null}
          </CardTitle>
          <CardDescription className="text-xs">{description}</CardDescription>
        </div>

        {!found || update?.outdated ? (
          <Button
            size="sm"
            className="shrink-0 gap-1.5"
            onClick={() => onInstall(kind)}
            disabled={busy}
          >
            <DownloadCloud className="size-4" />
            {!found ? "Install" : latest ? `Update to v${latest}` : "Update"}
          </Button>
        ) : upToDate ? (
          <span className="text-muted-foreground shrink-0 text-xs">Up to date</span>
        ) : null}
      </CardHeader>

      <CardContent className="flex min-w-0 items-center justify-between gap-2 px-4">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" size="sm" className="h-7 gap-1 px-2.5 text-xs">
              {sourceTriggerLabel(info)}
              <ChevronDown className="size-3.5" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            <DropdownMenuCheckboxItem
              checked={selected === "auto"}
              onSelect={() => {
                if (info?.source === "override") onClear(kind);
              }}
            >
              Auto-detected
            </DropdownMenuCheckboxItem>
            {managedPath ? (
              <DropdownMenuCheckboxItem
                checked={selected === "managed"}
                onSelect={() => onUseManaged(kind, managedPath)}
              >
                App-installed
              </DropdownMenuCheckboxItem>
            ) : null}
            <DropdownMenuSeparator />
            <DropdownMenuCheckboxItem
              checked={selected === "custom"}
              onSelect={() => onLocate(kind)}
            >
              Custom file…
            </DropdownMenuCheckboxItem>
          </DropdownMenuContent>
        </DropdownMenu>

        {update?.error ? (
          <span className="text-muted-foreground truncate text-xs" title={update.error}>
            Update check failed
          </span>
        ) : null}
      </CardContent>
    </Card>
  );
}
