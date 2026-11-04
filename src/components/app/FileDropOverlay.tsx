import { CircleAlert, CircleCheck, Loader2, Upload } from "lucide-react";

import { useFileDrop } from "@/hooks/useFileDrop";
import { DROP_FOLDER } from "@/lib/fileDrop";
import { cn } from "@/lib/utils";

/** Drop target covering the nearest positioned ancestor, plus a status pill while files are sent to `serial`. */
export function FileDropOverlay({ serial }: { serial: string | null }) {
  const { hovering, status } = useFileDrop(serial);

  return (
    <>
      {hovering ? (
        <div className="pointer-events-none absolute inset-2 z-50 flex flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed border-primary bg-background/85 px-6 text-center">
          <Upload className="size-7 text-primary" />
          {serial ? (
            <>
              <p className="text-sm font-medium">Drop to send to phone</p>
              <p className="text-xs text-muted-foreground">
                APKs are installed, other files go to {DROP_FOLDER}
              </p>
            </>
          ) : (
            <p className="text-sm font-medium">Connect a device to send files</p>
          )}
        </div>
      ) : null}
      {status ? (
        <div className="pointer-events-none absolute inset-x-0 bottom-5 z-50 flex justify-center px-4">
          <div
            role="status"
            className={cn(
              "flex max-w-full items-center gap-2 rounded-full border bg-popover px-3 py-1.5 text-xs text-popover-foreground shadow-lg",
              status.kind === "done" && status.failed && "border-destructive/60",
            )}
          >
            {status.kind === "sending" ? (
              <>
                <Loader2 className="size-3.5 shrink-0 animate-spin" />
                <span className="truncate">
                  {status.installing ? "Installing" : "Sending"} {status.name}
                  {status.total > 1 ? ` (${status.index}/${status.total})` : ""}…
                </span>
              </>
            ) : (
              <>
                {status.failed ? (
                  <CircleAlert className="size-3.5 shrink-0 text-destructive" />
                ) : (
                  <CircleCheck className="size-3.5 shrink-0 text-emerald-500" />
                )}
                <span className="truncate">{status.text}</span>
              </>
            )}
          </div>
        </div>
      ) : null}
    </>
  );
}
