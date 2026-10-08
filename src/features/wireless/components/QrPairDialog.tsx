import { useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, Loader2, QrCode, RotateCw } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import QRCode from "react-qr-code";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { errorMessage, ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { QrPayload } from "@/types";

const PAIR_TIMEOUT_SECS = 180;
const CLOSE_AFTER_SUCCESS_MS = 1200;

type Status =
  | { kind: "idle" }
  | { kind: "waiting" }
  | { kind: "success"; address: string }
  | { kind: "error"; message: string };

/** Header button that opens a small QR pairing popup and closes itself once paired. */
export function QrPairDialog() {
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [qr, setQr] = useState<QrPayload | null>(null);
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const [now, setNow] = useState(() => Date.now());
  // Each generated QR gets a session id so results from a stale pairing attempt
  // (closed popup, refreshed code) are ignored.
  const sessionRef = useRef(0);

  const generate = useCallback(async () => {
    const session = ++sessionRef.current;
    setQr(null);
    setStatus({ kind: "idle" });
    try {
      const payload = await ipc.wireless.qrPayload();
      if (session !== sessionRef.current) return;
      setQr(payload);
      setNow(Date.now());
      setStatus({ kind: "waiting" });
      const address = await ipc.wireless.qrPair(payload.payload, PAIR_TIMEOUT_SECS);
      queryClient.invalidateQueries({ queryKey: ["devices"] });
      if (session !== sessionRef.current) return;
      setStatus({ kind: "success", address });
      toast.success(`Connected to ${address}`);
    } catch (error) {
      if (session !== sessionRef.current) return;
      setStatus({ kind: "error", message: errorMessage(error) });
    }
  }, [queryClient]);

  function onOpenChange(next: boolean) {
    setOpen(next);
    if (next) {
      void generate();
    } else {
      sessionRef.current++;
      if (status.kind === "waiting") void ipc.wireless.cancelQr().catch(() => {});
      setQr(null);
      setStatus({ kind: "idle" });
    }
  }

  // Close automatically shortly after a successful connection.
  useEffect(() => {
    if (status.kind !== "success") return;
    const id = setTimeout(() => onOpenChange(false), CLOSE_AFTER_SUCCESS_MS);
    return () => clearTimeout(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status.kind]);

  // Tick once a second so the expiry counts down while the code is shown.
  useEffect(() => {
    if (!qr) return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [qr]);

  const secondsLeft = qr ? Math.max(0, qr.created + PAIR_TIMEOUT_SECS - Math.floor(now / 1000)) : 0;
  const timer = `${Math.floor(secondsLeft / 60)}:${String(secondsLeft % 60).padStart(2, "0")}`;

  return (
    <>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="size-9"
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
              On your phone, open Developer options › Wireless debugging › Pair device with QR
              code.
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
              {status.kind === "waiting" ? (
                <>
                  <p className="text-muted-foreground flex items-center gap-2 text-sm">
                    <Loader2 className="size-4 animate-spin" />
                    Waiting for your phone…
                  </p>
                  <p className="text-muted-foreground text-xs">
                    {secondsLeft > 0 ? `Expires in ${timer}` : "Code expired"}
                  </p>
                </>
              ) : null}
              {status.kind === "success" ? (
                <p className="text-success flex items-center gap-2 text-sm font-medium">
                  <CheckCircle2 className="size-4" />
                  Connected to {status.address}
                </p>
              ) : null}
              {status.kind === "error" ? (
                <>
                  <p className="text-destructive text-sm">{status.message}</p>
                  <Button variant="outline" size="sm" onClick={() => void generate()}>
                    <RotateCw className="size-3.5" />
                    Try again
                  </Button>
                </>
              ) : null}
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}
