import {
  Eraser,
  MonitorPlay,
  MoreVertical,
  Package,
  Play,
  Smartphone,
  Square,
  Trash2,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { AppInfo } from "@/types";

export function AppRow({
  app,
  busy,
  isOpen,
  onOpen,
  onOpenScreen,
  onStop,
  onLaunch,
  onClearData,
  onUninstall,
}: {
  app: AppInfo;
  busy: boolean;
  isOpen: boolean;
  onOpen: (app: AppInfo) => void;
  onOpenScreen: (app: AppInfo) => void;
  onStop: (app: AppInfo) => void;
  onLaunch: (app: AppInfo) => void;
  onClearData: (app: AppInfo) => void;
  onUninstall: (app: AppInfo) => void;
}) {
  return (
    <div className="flex h-14 w-full items-center gap-3 pr-2">
      <div className="bg-muted text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-md">
        <Package className="size-4" />
      </div>

      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <p className="truncate text-sm font-medium">{app.label}</p>
          {app.is_system ? (
            <Badge variant="secondary" className="shrink-0">
              System
            </Badge>
          ) : null}
          {isOpen ? (
            <Badge variant="success" className="shrink-0">
              Open
            </Badge>
          ) : null}
        </div>
        <p className="text-muted-foreground truncate font-mono text-xs">{app.package}</p>
      </div>

      <Button
        variant={isOpen ? "secondary" : "ghost"}
        size="icon"
        aria-label={isOpen ? `Close ${app.label} window` : `Open ${app.label} on the laptop`}
        title={isOpen ? "Close window" : "Open on the laptop"}
        disabled={busy}
        onClick={() => (isOpen ? onStop(app) : onOpen(app))}
      >
        {isOpen ? <Square /> : <MonitorPlay />}
      </Button>

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="icon" aria-label={`More actions for ${app.label}`}>
            <MoreVertical />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          {isOpen ? (
            <DropdownMenuItem onSelect={() => onStop(app)}>
              <Square />
              Close window
            </DropdownMenuItem>
          ) : (
            <DropdownMenuItem onSelect={() => onOpen(app)}>
              <MonitorPlay />
              Open in a window
            </DropdownMenuItem>
          )}
          <DropdownMenuItem onSelect={() => onOpenScreen(app)}>
            <Smartphone />
            Open in mirror mode
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => onLaunch(app)}>
            <Play />
            Launch on phone
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={() => onClearData(app)}>
            <Eraser />
            Clear data
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem variant="destructive" onSelect={() => onUninstall(app)}>
            <Trash2 />
            Uninstall
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
