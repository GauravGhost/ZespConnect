import { DownloadCloud, FolderSearch, RotateCcw, Terminal } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { ToolInfo, ToolKind, UpdateInfo } from "@/types";

const SOURCE_LABEL: Record<ToolInfo["source"], string> = {
  override: "Custom path",
  path: "Auto-detected · PATH",
  known: "Auto-detected",
  not_found: "Not found",
};

export function ToolCard({
  kind,
  title,
  description,
  info,
  update,
  busy,
  onLocate,
  onClear,
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
  onInstall: (kind: ToolKind) => void;
}) {
  const found = Boolean(info?.found);
  const outdated = Boolean(update?.outdated);
  const latest = update?.latest_version ?? null;

  return (
    <Card className="min-w-0">
      <CardHeader className="min-w-0">
        <div className="flex items-center justify-between gap-2">
          <CardTitle className="flex min-w-0 items-center gap-2 text-base">
            <Terminal className="size-4 shrink-0" />
            <span className="truncate">{title}</span>
          </CardTitle>
          <Badge variant={found && !outdated ? "success" : "warning"} className="shrink-0">
            {!found
              ? "Not found"
              : outdated
                ? `Update available${latest ? ` · v${latest}` : ""}`
                : found && update?.latest_version
                  ? "Up to date"
                  : `Found${info?.version ? ` · v${info.version}` : ""}`}
          </Badge>
        </div>
        <CardDescription>{description}</CardDescription>
      </CardHeader>

      <CardContent className="flex min-w-0 flex-1 flex-col gap-4">
        <div className="min-w-0 space-y-1">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="secondary">{info ? SOURCE_LABEL[info.source] : "Checking…"}</Badge>
            {found && info?.version ? (
              <span className="text-muted-foreground text-xs">installed v{info.version}</span>
            ) : null}
          </div>
          <p
            className="text-muted-foreground font-mono text-xs break-all"
            title={info?.path ?? undefined}
          >
            {info?.path ?? "No binary resolved yet"}
          </p>
          {update?.error ? (
            <p className="text-muted-foreground text-xs">Update check failed - {update.error}</p>
          ) : null}
        </div>

        <div className="mt-auto flex flex-wrap gap-2">
          <Button variant="outline" size="sm" className="gap-2" onClick={() => onLocate(kind)}>
            <FolderSearch className="size-4" />
            {found ? "Change location…" : "Locate binary…"}
          </Button>

          {info?.source === "override" ? (
            <Button variant="ghost" size="sm" className="gap-2" onClick={() => onClear(kind)}>
              <RotateCcw className="size-4" />
              Use auto-detected
            </Button>
          ) : null}

          {!found ? (
            <Button size="sm" className="gap-2" onClick={() => onInstall(kind)} disabled={busy}>
              <DownloadCloud className="size-4" />
              Install
            </Button>
          ) : outdated ? (
            <Button size="sm" className="gap-2" onClick={() => onInstall(kind)} disabled={busy}>
              <DownloadCloud className="size-4" />
              {latest ? `Update to v${latest}` : "Update"}
            </Button>
          ) : null}
        </div>
      </CardContent>
    </Card>
  );
}
