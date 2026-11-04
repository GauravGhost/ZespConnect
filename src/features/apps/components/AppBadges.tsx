import { Badge } from "@/components/ui/badge";
import type { AppInfo } from "@/types";

export function AppBadges({ app, isOpen }: { app: AppInfo; isOpen: boolean }) {
  return (
    <>
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
    </>
  );
}
