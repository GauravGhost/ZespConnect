import { useMutation, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2, Loader2, Smartphone, Wifi } from "lucide-react";
import { useState } from "react";
import QRCode from "react-qr-code";
import { toast } from "sonner";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useDevices } from "@/features/devices";
import { errorMessage, ipc } from "@/lib/ipc";
import { stateLabel, transportLabel } from "@/lib/format";
import type { QrPayload } from "@/types";

export function WirelessPanel() {
  const queryClient = useQueryClient();
  const { data: devices = [] } = useDevices();
  const [qr, setQr] = useState<QrPayload | null>(null);
  const [generating, setGenerating] = useState(false);

  const pair = useMutation({
    mutationFn: (payload: string) => ipc.wireless.qrPair(payload, 180),
    onSuccess: (address) => {
      toast.success(`Connected to ${address}`);
      queryClient.invalidateQueries({ queryKey: ["devices"] });
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  async function start() {
    pair.reset();
    setGenerating(true);
    try {
      const payload = await ipc.wireless.qrPayload();
      setQr(payload);
      pair.mutate(payload.payload);
    } catch (error) {
      toast.error(errorMessage(error));
      setQr(null);
    } finally {
      setGenerating(false);
    }
  }

  function reset() {
    pair.reset();
    setQr(null);
  }

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6 p-6">
      <div>
        <h1 className="text-lg font-semibold">Wireless</h1>
        <p className="text-muted-foreground text-sm">
          Pair a phone over Wi-Fi by scanning a QR code — no USB cable needed.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-sm">
            <Wifi className="size-4" />
            Pair over Wi-Fi (QR)
          </CardTitle>
          <CardDescription>
            On the phone: Developer options → Wireless debugging → Pair device with QR code. The
            phone and this computer must be on the same network.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {qr ? (
            <div className="flex flex-col items-center gap-4">
              <div className="rounded-lg bg-white p-4 shadow-sm">
                <QRCode value={qr.payload} size={200} bgColor="#ffffff" fgColor="#000000" />
              </div>

              {pair.isPending ? (
                <p className="text-muted-foreground flex items-center gap-2 text-sm">
                  <Loader2 className="size-4 animate-spin" />
                  Waiting for your phone to scan…
                </p>
              ) : null}

              {pair.isSuccess ? (
                <p className="text-success flex items-center gap-2 text-sm">
                  <CheckCircle2 className="size-4" />
                  Connected to {pair.data}
                </p>
              ) : null}

              {pair.isError ? (
                <Alert variant="destructive" className="w-full">
                  <AlertTriangle />
                  <AlertTitle>Pairing failed</AlertTitle>
                  <AlertDescription>{errorMessage(pair.error)}</AlertDescription>
                </Alert>
              ) : null}

              <div className="flex gap-2">
                <Button variant="outline" onClick={reset}>
                  {pair.isSuccess ? "Done" : "Cancel"}
                </Button>
                {pair.isSuccess || pair.isError ? (
                  <Button onClick={start} disabled={generating}>
                    Pair another device
                  </Button>
                ) : null}
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center gap-3 py-4 text-center">
              <p className="text-muted-foreground max-w-md text-sm">
                Generate a pairing code, then scan it from your phone's Wireless debugging screen.
              </p>
              <Button onClick={start} disabled={generating}>
                {generating ? (
                  <Loader2 className="size-4 animate-spin" />
                ) : (
                  <Wifi className="size-4" />
                )}
                Generate pairing QR
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-sm">
            <Smartphone className="size-4" />
            Connected devices
            <Badge variant="secondary">{devices.length}</Badge>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {devices.length === 0 ? (
            <p className="text-muted-foreground text-sm">
              No devices yet. Pair one above, or connect over USB.
            </p>
          ) : (
            <ul className="divide-y">
              {devices.map((device) => (
                <li key={device.serial} className="flex items-center justify-between gap-3 py-2">
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium">
                      {device.display_name || device.model || device.serial}
                    </p>
                    <p className="text-muted-foreground truncate font-mono text-xs">
                      {device.serial}
                    </p>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    <Badge variant={device.transport === "wifi" ? "secondary" : "outline"}>
                      {transportLabel(device)}
                    </Badge>
                    <Badge variant={device.connected ? "success" : "warning"}>
                      {stateLabel(device.state)}
                    </Badge>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
