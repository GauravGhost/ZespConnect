import { Package } from "lucide-react";

import { AppActionsMenu, AppWindowButton, type AppItemProps } from "./AppActionsMenu";
import { AppBadges } from "./AppBadges";

export function AppRow(props: AppItemProps) {
  const { app, isOpen } = props;
  return (
    <div className="flex h-14 w-full items-center gap-3 pr-2">
      <div className="bg-muted text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-md">
        <Package className="size-4" />
      </div>

      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <p className="truncate text-sm font-medium">{app.label}</p>
          <AppBadges app={app} isOpen={isOpen} />
        </div>
        <p className="text-muted-foreground truncate font-mono text-xs">{app.package}</p>
      </div>

      <AppWindowButton {...props} />
      <AppActionsMenu {...props} />
    </div>
  );
}
