import { Loader2, QrCode, RotateCw } from "lucide-react";
import { useEffect, useState } from "react";
import QRCode from "react-qr-code";

import { QrPairStatusMessage } from "@/components/app/QrPairStatus";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useQrPairing } from "@/hooks/useQrPairing";
import { cn } from "@/lib/utils";

const CLOSE_AFTER_SUCCESS_MS = 1200;

/** Header button that opens a small QR pairing popup and closes itself once paired. */
export function QrPairDialog() {
  const [open, setOpen] = useState(false);
  const { qr, status, secondsLeft, timer, generate, cancel } = useQrPairing();

  function onOpenChange(next: boolean) {
    setOpen(next);
    if (next) void generate();
    else cancel();
  }

  // Close automatically shortly after a successful connection.
  useEffect(() => {
    if (status.kind !== "success") return;
    const id = setTimeout(() => onOpenChange(false), CLOSE_AFTER_SUCCESS_MS);
    return () => clearTimeout(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status.kind]);

  return (
    <>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="size-8 rounded-lg"
            aria-label="Pair with QR code"
            onClick={() => onOpenChange(true)}
          >
            <QrCode />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Pair with QR code</TooltipContent>
      </Tooltip>

      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="gap-5 p-6 sm:max-w-xs">
          <DialogHeader className="text-center sm:text-center">
            <DialogTitle>Scan to connect</DialogTitle>
            <DialogDescription>
              On your phone, open Developer options › Wireless debugging › Pair device with QR code.
            </DialogDescription>
          </DialogHeader>

          <div className="flex flex-col items-center gap-4">
            <div className="flex size-[192px] items-center justify-center rounded-2xl bg-white p-3 shadow-sm">
              {qr ? (
                <QRCode
                  value={qr.payload}
                  size={168}
                  bgColor="#ffffff"
                  fgColor="#000000"
                  className={cn(status.kind === "success" && "opacity-30")}
                />
              ) : (
                <Loader2 className="size-6 animate-spin text-neutral-400" />
              )}
            </div>

            <div className="flex min-h-10 flex-col items-center gap-1 text-center">
              <QrPairStatusMessage status={status} />
              {status.kind === "waiting" ? (
                <p className="text-muted-foreground text-xs">
                  {secondsLeft > 0 ? `Expires in ${timer}` : "Code expired"}
                </p>
              ) : null}
              {status.kind === "error" ? (
                <Button variant="outline" size="sm" onClick={() => void generate()}>
                  <RotateCw className="size-3.5" />
                  Try again
                </Button>
              ) : null}
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}
