import { Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { AppInfo } from "@/types";

export type AppConfirmAction = "clearData" | "uninstall";

const COPY: Record<
  AppConfirmAction,
  { title: string; confirm: string; body: (app: AppInfo) => string }
> = {
  clearData: {
    title: "Clear app data?",
    confirm: "Clear data",
    body: (app) => `All data for ${app.label} will be erased. This cannot be undone.`,
  },
  uninstall: {
    title: "Uninstall app?",
    confirm: "Uninstall",
    body: (app) => `${app.label} will be removed from the device.`,
  },
};

export function AppConfirmDialog({
  app,
  action,
  pending,
  onOpenChange,
  onConfirm,
}: {
  app: AppInfo | null;
  action: AppConfirmAction;
  pending: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}) {
  const copy = COPY[action];

  return (
    <Dialog open={app !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{copy.title}</DialogTitle>
          <DialogDescription>{app ? copy.body(app) : ""}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={onConfirm} disabled={pending}>
            {pending ? <Loader2 className="size-4 animate-spin" /> : null}
            {copy.confirm}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
