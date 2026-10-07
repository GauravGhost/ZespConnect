import { DownloadCloud, FolderSearch, RotateCcw, Terminal } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { ToolInfo, ToolKind } from "@/types";

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
  busy,
  onLocate,
  onClear,
  onInstall,
}: {
  kind: ToolKind;
  title: string;
  description: string;
  info: ToolInfo | undefined;
  busy: boolean;
  onLocate: (kind: ToolKind) => void;
  onClear: (kind: ToolKind) => void;
  onInstall: (kind: ToolKind) => void;
}) {
  const found = Boolean(info?.found);

  return (
    <Card className="min-w-0">
      <CardHeader className="min-w-0">
        <div className="flex items-center justify-between gap-2">
          <CardTitle className="flex min-w-0 items-center gap-2 text-base">
            <Terminal className="size-4 shrink-0" />
            <span className="truncate">{title}</span>
          </CardTitle>
          <Badge variant={found ? "success" : "warning"} className="shrink-0">
            {found ? `Found${info?.version ? ` · v${info.version}` : ""}` : "Not found"}
          </Badge>
        </div>
        <CardDescription>{description}</CardDescription>
      </CardHeader>

      <CardContent className="min-w-0 space-y-4">
        <div className="min-w-0 space-y-1">
          <div className="flex items-center gap-2">
            <Badge variant="secondary">{info ? SOURCE_LABEL[info.source] : "Checking…"}</Badge>
          </div>
          <p
            className="text-muted-foreground font-mono text-xs break-all"
            title={info?.path ?? undefined}
          >
            {info?.path ?? "No binary resolved yet"}
          </p>
        </div>

        <div className="flex flex-wrap gap-2">
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
          ) : null}
        </div>
      </CardContent>
    </Card>
  );
}
