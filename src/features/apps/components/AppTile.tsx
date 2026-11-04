import { Package } from "lucide-react";

import { AppActionsMenu, AppWindowButton, type AppItemProps } from "./AppActionsMenu";
import { AppBadges } from "./AppBadges";

export function AppTile(props: AppItemProps) {
  const { app, isOpen } = props;
  return (
    <div className="bg-card flex flex-col rounded-lg border p-3">
      <div className="flex items-start justify-between gap-2">
        <div className="bg-muted text-muted-foreground flex size-10 shrink-0 items-center justify-center rounded-md">
          <Package className="size-5" />
        </div>
        <div className="flex shrink-0 items-center">
          <AppWindowButton {...props} />
          <AppActionsMenu {...props} />
        </div>
      </div>

      <div className="mt-2 flex items-center gap-1.5">
        <p className="truncate text-sm font-medium" title={app.label}>
          {app.label}
        </p>
        <AppBadges app={app} isOpen={isOpen} />
      </div>
      <p className="text-muted-foreground truncate font-mono text-xs" title={app.package}>
        {app.package}
      </p>
    </div>
  );
}
