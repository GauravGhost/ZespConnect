import {
  Eraser,
  FileDown,
  MonitorPlay,
  MoreVertical,
  Play,
  Smartphone,
  Square,
  Trash2,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { AppInfo } from "@/types";

export interface AppActionHandlers {
  onOpen: (app: AppInfo) => void;
  onOpenScreen: (app: AppInfo) => void;
  onStop: (app: AppInfo) => void;
  onLaunch: (app: AppInfo) => void;
  onExportApk: (app: AppInfo) => void;
  onClearData: (app: AppInfo) => void;
  onUninstall: (app: AppInfo) => void;
}

export interface AppItemProps extends AppActionHandlers {
  app: AppInfo;
  busy: boolean;
  isOpen: boolean;
}

/** Toggles the app's own mirror window: opens one, or closes the open one. */
export function AppWindowButton({
  app,
  busy,
  isOpen,
  onOpen,
  onStop,
}: Pick<AppItemProps, "app" | "busy" | "isOpen" | "onOpen" | "onStop">) {
  return (
    <Button
      variant={isOpen ? "secondary" : "ghost"}
      size="icon"
      aria-label={isOpen ? `Close ${app.label} window` : `Open ${app.label} in a window`}
      title={isOpen ? "Close window" : "Open in a window"}
      disabled={busy}
      onClick={() => (isOpen ? onStop(app) : onOpen(app))}
    >
      {isOpen ? <Square /> : <MonitorPlay />}
    </Button>
  );
}

export function AppActionsMenu({
  app,
  isOpen,
  onOpen,
  onOpenScreen,
  onStop,
  onLaunch,
  onExportApk,
  onClearData,
  onUninstall,
}: Omit<AppItemProps, "busy">) {
  return (
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
        <DropdownMenuItem onSelect={() => onExportApk(app)}>
          <FileDown />
          Export APK
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
  );
}
