import { Loader2, QrCode, RotateCw } from "lucide-react";
import QRCode from "react-qr-code";

import { QrPairStatusMessage } from "@/components/app/QrPairStatus";
import { Button } from "@/components/ui/button";
import type { QrPairing } from "@/hooks/useQrPairing";
import { cn } from "@/lib/utils";

/** The pairing state is owned by the panel so it survives switching tabs. */
export function QrPairTab({ pairing }: { pairing: QrPairing }) {
  const { qr, status, generating, secondsLeft, timer, generate, cancel } = pairing;

  if (!qr) {
    return (
      <div className="flex min-h-[248px] flex-col items-center justify-center gap-3 rounded-2xl border border-dashed border-border px-6 py-10 text-center">
        <QrCode className="text-muted-foreground size-7" />
        <div className="space-y-1">
          <p className="text-sm font-medium">Pair with a QR code</p>
          <p className="text-muted-foreground text-xs">
            We'll generate the code only when you're ready to scan.
          </p>
        </div>
        <Button size="sm" onClick={generate} disabled={generating}>
          {generating ? <Loader2 className="size-4 animate-spin" /> : <QrCode className="size-4" />}
          Show QR code
        </Button>
      </div>
    );
  }

  return (
    <div className="@container flex min-h-[248px] items-center">
      <div className="flex w-full flex-col items-center gap-5 @sm:flex-row @sm:items-start">
        <div className="shrink-0 rounded-2xl bg-white p-3 shadow-sm">
          <QRCode value={qr.payload} size={168} bgColor="#ffffff" fgColor="#000000" />
        </div>
        <div className="min-w-0 flex-1 space-y-3">
          <div className="space-y-1">
            <p className="text-sm font-semibold">Scan to connect</p>
            <p className="text-muted-foreground text-xs">
              Open your phone's Wireless debugging screen and scan this QR code.
            </p>
          </div>

          <QrPairStatusMessage status={status} />

          <div className="flex flex-wrap items-center gap-2">
            <Button variant="outline" size="sm" onClick={generate} disabled={generating}>
              <RotateCw className={cn("size-3.5", generating && "animate-spin")} />
              Refresh
            </Button>
            <Button variant="ghost" size="sm" onClick={cancel}>
              Hide
            </Button>
          </div>
          <p className="text-muted-foreground text-xs">
            {secondsLeft > 0
              ? `QR code expires in ${timer}`
              : "QR code expired - refresh to generate a new one."}
          </p>
        </div>
      </div>
    </div>
  );
}
