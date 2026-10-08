import { useQueryClient } from "@tanstack/react-query";
import {
  CheckCircle2,
  Clock,
  CodeXml,
  Copy,
  KeyRound,
  Loader2,
  MoreVertical,
  Plug,
  QrCode,
  RefreshCw,
  RotateCw,
  Unplug,
  Usb,
  Wifi,
  X,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import QRCode from "react-qr-code";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useDevices } from "@/features/devices";
import { stateLabel, timeAgo } from "@/lib/format";
import { errorMessage, ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { Device, MdnsDevice, QrPayload } from "@/types";

import {
  mdnsKey,
  useConnectWireless,
  useDisconnectWireless,
  useEnableTcpip,
  useMdnsServices,
  usePairCode,
  usePairQr,
  useRememberSavedDevices,
} from "../hooks";
import { useSavedWireless, type SavedWirelessDevice } from "../saved";

const PAIRING_SERVICE = "_adb-tls-pairing._tcp";
const CONNECT_SERVICE = "_adb-tls-connect._tcp";
const PAIR_TIMEOUT_SECS = 180;

function deviceName(device: Device): string {
  return device.display_name || device.model || device.serial;
}

export function WirelessPanel() {
  const queryClient = useQueryClient();
  const { data: devices = [], isFetching } = useDevices();
  const { data: services = [] } = useMdnsServices();
  const saved = useSavedWireless((state) => state.devices);
  const forget = useSavedWireless((state) => state.forget);
  useRememberSavedDevices();

  const [method, setMethod] = useState("qr");
  const [addr, setAddr] = useState("");
  const [code, setCode] = useState("");
  const [qr, setQr] = useState<QrPayload | null>(null);
  const [generating, setGenerating] = useState(false);
  const [now, setNow] = useState(() => Date.now());

  const pairQr = usePairQr();
  const pairCode = usePairCode();
  const connect = useConnectWireless();
  const disconnect = useDisconnectWireless();
  const enable = useEnableTcpip();

  const usb = devices.filter((device) => device.transport === "usb");
  const wireless = devices.filter((device) => device.transport === "wifi");
  const discovered = services.filter(
    (service) =>
      service.service_type === CONNECT_SERVICE || service.service_type === PAIRING_SERVICE,
  );

  const pending = pairCode.isPending || connect.isPending;
  const codeValid = code.trim() === "" || /^\d{6,16}$/.test(code.trim());
  const canSubmit = addr.trim().length > 0 && codeValid && !pending;
  const isConnected = (value: string) =>
    devices.some((device) => device.serial === value || device.address === value);

  const copy = useCallback(async (value: string, label: string) => {
    try {
      await navigator.clipboard.writeText(value);
      toast.success(`${label} copied`);
    } catch {
      toast.error("Could not copy to clipboard");
    }
  }, []);

  // Tick once a second so the QR expiry counts down while it is on screen.
  useEffect(() => {
    if (!qr) return;
    setNow(Date.now());
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [qr]);

  function submit(event: React.FormEvent) {
    event.preventDefault();
    const target = addr.trim();
    const secret = code.trim();
    if (!target || !codeValid) return;
    // Address + code pairs first; a bare address just connects.
    if (secret) pairCode.mutate({ addr: target, code: secret });
    else connect.mutate(target);
  }

  const generate = useCallback(async () => {
    pairQr.reset();
    setGenerating(true);
    try {
      const payload = await ipc.wireless.qrPayload();
      setQr(payload);
      setNow(Date.now());
      pairQr.mutate(payload.payload);
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setGenerating(false);
    }
  }, [pairQr]);

  function hideQr() {
    pairQr.reset();
    setQr(null);
  }

  function refresh() {
    queryClient.invalidateQueries({ queryKey: ["devices"] });
    queryClient.invalidateQueries({ queryKey: mdnsKey });
  }

  const secondsLeft = qr ? Math.max(0, qr.created + PAIR_TIMEOUT_SECS - Math.floor(now / 1000)) : 0;
  const timer = `${Math.floor(secondsLeft / 60)}:${String(secondsLeft % 60).padStart(2, "0")}`;

  return (
    <div className="mx-auto w-full max-w-5xl p-4">
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <Card className="gap-4 py-5">
          <CardHeader className="gap-1 px-5">
            <CardTitle className="text-base">Connect a device</CardTitle>
            <CardDescription>Choose a method to connect your phone.</CardDescription>
          </CardHeader>
          <CardContent className="px-5">
            <Tabs value={method} onValueChange={setMethod} className="gap-4">
              <TabsList className="w-full [&>button]:px-2">
                <TabsTrigger value="qr">
                  <QrCode />
                  QR code
                </TabsTrigger>
                <TabsTrigger value="manual">
                  <CodeXml />
                  Pair code
                </TabsTrigger>
                <TabsTrigger value="usb">
                  <Usb />
                  USB
                </TabsTrigger>
              </TabsList>

              <TabsContent value="qr">
                {qr ? (
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

                        {pairQr.isPending ? (
                          <p className="text-muted-foreground flex items-center gap-2 text-sm">
                            <Loader2 className="size-4 animate-spin" />
                            Waiting for your phone…
                          </p>
                        ) : null}
                        {pairQr.isSuccess ? (
                          <p className="text-success flex items-center gap-2 text-sm">
                            <CheckCircle2 className="size-4" />
                            Connected to {pairQr.data}
                          </p>
                        ) : null}
                        {pairQr.isError ? (
                          <p className="text-destructive text-sm">{errorMessage(pairQr.error)}</p>
                        ) : null}

                        <div className="flex flex-wrap items-center gap-2">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={generate}
                            disabled={generating}
                          >
                            <RotateCw className={cn("size-3.5", generating && "animate-spin")} />
                            Refresh
                          </Button>
                          <Button variant="ghost" size="sm" onClick={hideQr}>
                            Hide
                          </Button>
                        </div>
                        <p className="text-muted-foreground text-xs">
                          {secondsLeft > 0
                            ? `QR code expires in ${timer}`
                            : "QR code expired — refresh to generate a new one."}
                        </p>
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="flex min-h-[248px] flex-col items-center justify-center gap-3 rounded-2xl border border-dashed border-border px-6 py-10 text-center">
                    <QrCode className="text-muted-foreground size-7" />
                    <div className="space-y-1">
                      <p className="text-sm font-medium">Pair with a QR code</p>
                      <p className="text-muted-foreground text-xs">
                        We'll generate the code only when you're ready to scan.
                      </p>
                    </div>
                    <Button size="sm" onClick={generate} disabled={generating}>
                      {generating ? (
                        <Loader2 className="size-4 animate-spin" />
                      ) : (
                        <QrCode className="size-4" />
                      )}
                      Show QR code
                    </Button>
                  </div>
                )}
              </TabsContent>

              <TabsContent value="manual" className="space-y-3">
                <form className="space-y-3" onSubmit={submit}>
                  <div className="space-y-1.5">
                    <Label htmlFor="wireless-addr" className="text-muted-foreground text-xs">
                      Address
                    </Label>
                    <Input
                      id="wireless-addr"
                      value={addr}
                      onChange={(event) => setAddr(event.target.value)}
                      placeholder="192.168.1.42:5555"
                      className="h-9 font-mono"
                      autoComplete="off"
                      spellCheck={false}
                    />
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="wireless-code" className="text-muted-foreground text-xs">
                      Pairing code <span className="font-normal">(optional)</span>
                    </Label>
                    <Input
                      id="wireless-code"
                      value={code}
                      onChange={(event) =>
                        setCode(event.target.value.replace(/\D/g, "").slice(0, 16))
                      }
                      placeholder="6-digit code"
                      inputMode="numeric"
                      className="h-9 font-mono"
                      autoComplete="off"
                    />
                  </div>
                  <Button type="submit" size="sm" className="w-full" disabled={!canSubmit}>
                    {pending ? (
                      <Loader2 className="size-4 animate-spin" />
                    ) : code.trim() ? (
                      <KeyRound className="size-4" />
                    ) : (
                      <Plug className="size-4" />
                    )}
                    {code.trim() ? "Pair & connect" : "Connect"}
                  </Button>
                </form>
                <p className="text-muted-foreground text-xs">
                  Enter the phone's address to connect, or add its code to pair first.
                </p>

                {discovered.length > 0 ? (
                  <div className="pt-1">
                    <p className="text-muted-foreground mb-1 text-xs font-medium tracking-wide uppercase">
                      Discovered
                    </p>
                    <ul className="divide-y divide-border/60">
                      {discovered.map((service) => (
                        <ServiceRow
                          key={`${service.name}-${service.address}`}
                          service={service}
                          connected={isConnected(service.address)}
                          onUse={(value) => setAddr(value)}
                          onConnect={(value) => connect.mutate(value)}
                          connecting={connect.isPending && connect.variables === service.address}
                        />
                      ))}
                    </ul>
                  </div>
                ) : null}
              </TabsContent>

              <TabsContent value="usb">
                {usb.length === 0 ? (
                  <p className="text-muted-foreground text-sm">
                    Connect a phone over USB to switch it to Wi-Fi.
                  </p>
                ) : (
                  <ul className="divide-y divide-border/60">
                    {usb.map((device) => {
                      const busy = enable.isPending && enable.variables?.serial === device.serial;
                      return (
                        <li key={device.serial} className="flex items-center gap-3 py-2">
                          <div className="min-w-0 flex-1">
                            <p className="truncate text-sm font-medium">{deviceName(device)}</p>
                            <p className="text-muted-foreground truncate font-mono text-[11px]">
                              {device.serial}
                            </p>
                          </div>
                          <Button
                            variant="outline"
                            size="sm"
                            disabled={busy}
                            onClick={() => enable.mutate({ serial: device.serial })}
                          >
                            {busy ? (
                              <Loader2 className="size-3.5 animate-spin" />
                            ) : (
                              <Wifi className="size-3.5" />
                            )}
                            Enable wireless
                          </Button>
                        </li>
                      );
                    })}
                  </ul>
                )}
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>

        <Card className="gap-4 py-5">
          <CardHeader className="flex-row items-start justify-between gap-3 px-5">
            <div className="space-y-1">
              <CardTitle className="text-base">Connected devices</CardTitle>
              <CardDescription>Manage your connected phones.</CardDescription>
            </div>
            <Button
              variant="ghost"
              size="icon"
              className="size-8 shrink-0"
              onClick={refresh}
              aria-label="Refresh"
              title="Refresh"
            >
              <RefreshCw className={cn("size-4", isFetching && "animate-spin")} />
            </Button>
          </CardHeader>
          <CardContent className="space-y-5 px-5">
            {wireless.length === 0 ? (
              <p className="text-muted-foreground text-sm">No phones connected over Wi-Fi yet.</p>
            ) : (
              <ul className="space-y-3">
                {wireless.map((device) => {
                  const target = device.address ?? device.serial;
                  const busy = disconnect.isPending && disconnect.variables === target;
                  return (
                    <li
                      key={device.serial}
                      className="border-success/40 bg-success/5 rounded-2xl border p-3"
                    >
                      <div className="flex items-start gap-3">
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <p className="truncate text-sm font-semibold">{deviceName(device)}</p>
                            <Badge variant={device.connected ? "success" : "warning"}>
                              {stateLabel(device.state)}
                            </Badge>
                          </div>
                          <p className="text-muted-foreground mt-1 truncate font-mono text-[11px]">
                            {device.serial}
                          </p>
                          {device.address ? (
                            <p className="text-muted-foreground truncate font-mono text-[11px]">
                              {device.address}
                            </p>
                          ) : null}
                        </div>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="size-8 shrink-0"
                              aria-label="Device actions"
                            >
                              <MoreVertical className="size-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem
                              onSelect={() => {
                                void copy(device.serial, "Code");
                              }}
                            >
                              <Copy />
                              Copy code
                            </DropdownMenuItem>
                            {device.address ? (
                              <DropdownMenuItem
                                onSelect={() => {
                                  void copy(device.address as string, "Address");
                                }}
                              >
                                <Copy />
                                Copy address
                              </DropdownMenuItem>
                            ) : null}
                            <DropdownMenuSeparator />
                            <DropdownMenuItem
                              variant="destructive"
                              disabled={busy}
                              onSelect={() => disconnect.mutate(target)}
                            >
                              <Unplug />
                              Disconnect
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </div>

                      <div className="mt-3 flex flex-wrap gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          disabled={!device.address}
                          onClick={() => {
                            if (device.address) void copy(device.address, "Address");
                          }}
                        >
                          <Copy className="size-3.5" />
                          Copy address
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => {
                            void copy(device.serial, "Code");
                          }}
                        >
                          <CodeXml className="size-3.5" />
                          Copy code
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          className="text-destructive hover:bg-destructive/10 hover:text-destructive"
                          disabled={busy}
                          onClick={() => disconnect.mutate(target)}
                        >
                          {busy ? (
                            <Loader2 className="size-3.5 animate-spin" />
                          ) : (
                            <Unplug className="size-3.5" />
                          )}
                          Disconnect
                        </Button>
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}

            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <Clock className="text-muted-foreground size-4 shrink-0" />
                <div>
                  <p className="text-sm font-medium">Saved devices</p>
                  <p className="text-muted-foreground text-xs">Previously connected devices.</p>
                </div>
              </div>

              {saved.length === 0 ? (
                <p className="text-muted-foreground text-sm">
                  Phones you connect over Wi-Fi will be saved here.
                </p>
              ) : (
                <ul className="space-y-2">
                  {saved.map((entry) => (
                    <SavedRow
                      key={entry.serial}
                      entry={entry}
                      connected={isConnected(entry.serial)}
                      onConnect={() => connect.mutate(entry.address ?? entry.serial)}
                      connecting={
                        connect.isPending && connect.variables === (entry.address ?? entry.serial)
                      }
                      onForget={() => forget(entry.serial)}
                    />
                  ))}
                </ul>
              )}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function ServiceRow({
  service,
  connected,
  connecting,
  onUse,
  onConnect,
}: {
  service: MdnsDevice;
  connected: boolean;
  connecting: boolean;
  onUse: (address: string) => void;
  onConnect: (address: string) => void;
}) {
  const pairing = service.service_type === PAIRING_SERVICE;
  return (
    <li className="flex items-center gap-3 py-2">
      <div className="min-w-0 flex-1">
        <p className="truncate text-xs font-medium">{service.name}</p>
        <p className="text-muted-foreground truncate font-mono text-[11px]">{service.address}</p>
      </div>
      {pairing ? (
        <Button variant="ghost" size="sm" onClick={() => onUse(service.address)}>
          Use address
        </Button>
      ) : (
        <Button
          variant="outline"
          size="sm"
          disabled={connected || connecting}
          onClick={() => onConnect(service.address)}
        >
          {connecting ? (
            <Loader2 className="size-3.5 animate-spin" />
          ) : (
            <Plug className="size-3.5" />
          )}
          {connected ? "Connected" : "Connect"}
        </Button>
      )}
    </li>
  );
}

function SavedRow({
  entry,
  connected,
  connecting,
  onConnect,
  onForget,
}: {
  entry: SavedWirelessDevice;
  connected: boolean;
  connecting: boolean;
  onConnect: () => void;
  onForget: () => void;
}) {
  return (
    <li className="flex items-center gap-3 rounded-2xl border border-border/60 p-3">
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">{entry.name}</p>
        <p className="text-muted-foreground truncate font-mono text-[11px]">{entry.serial}</p>
        {entry.address ? (
          <p className="text-muted-foreground truncate font-mono text-[11px]">{entry.address}</p>
        ) : null}
      </div>
      <div className="shrink-0 text-right">
        <p className="text-muted-foreground text-[11px]">Last connected</p>
        <p className="text-xs">{timeAgo(entry.lastConnected)}</p>
      </div>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="size-8 shrink-0"
            aria-label="Saved device actions"
          >
            <MoreVertical className="size-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem disabled={connected || connecting} onSelect={onConnect}>
            {connecting ? <Loader2 className="size-4 animate-spin" /> : <Plug />}
            Connect
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem variant="destructive" onSelect={onForget}>
            <X />
            Forget
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </li>
  );
}
