import { CheckCircle2, Loader2 } from "lucide-react";

import type { QrPairStatus } from "@/hooks/useQrPairing";

/** One-line progress of a QR pairing attempt; renders nothing while idle. */
export function QrPairStatusMessage({ status }: { status: QrPairStatus }) {
  if (status.kind === "waiting") {
    return (
      <p className="text-muted-foreground flex items-center gap-2 text-sm">
        <Loader2 className="size-4 animate-spin" />
        Waiting for your phone…
      </p>
    );
  }
  if (status.kind === "success") {
    return (
      <p className="text-success flex items-center gap-2 text-sm font-medium">
        <CheckCircle2 className="size-4" />
        Connected to {status.address}
      </p>
    );
  }
  if (status.kind === "error") {
    return <p className="text-destructive text-sm">{status.message}</p>;
  }
  return null;
}
