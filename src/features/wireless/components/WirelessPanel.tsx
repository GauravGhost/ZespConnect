import { useQueryClient } from "@tanstack/react-query";
import {
  Clock,
  CodeXml,
  Loader2,
  MoreVertical,
  Plug,
  QrCode,
  RefreshCw,
  Usb,
  X,
} from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { devicesKey, useDevices } from "@/hooks/useDevices";
import { useQrPairing } from "@/hooks/useQrPairing";
import { timeAgo } from "@/lib/format";
import { cn } from "@/lib/utils";

import {
  mdnsKey,
  useConnectWireless,
  useDisconnectWireless,
  useEnableTcpip,
  useMdnsServices,
  usePairCode,
  useRememberSavedDevices,
} from "../hooks";
import { useSavedWireless, type SavedWirelessDevice } from "../saved";
import { PairCodeTab } from "./PairCodeTab";
import { QrPairTab } from "./QrPairTab";
import { UsbTab } from "./UsbTab";
import { WirelessDeviceRow } from "./WirelessDeviceRow";

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

  const pairing = useQrPairing();
  const pairCode = usePairCode();
  const connect = useConnectWireless();
  const disconnect = useDisconnectWireless();
  const enable = useEnableTcpip();

  const usb = devices.filter((device) => device.transport === "usb");
  const wireless = devices.filter((device) => device.transport === "wifi");

  const isConnected = (value: string) =>
    devices.some((device) => device.serial === value || device.address === value);

  function refresh() {
    queryClient.invalidateQueries({ queryKey: devicesKey });
    queryClient.invalidateQueries({ queryKey: mdnsKey });
  }

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
                <QrPairTab pairing={pairing} />
              </TabsContent>

              <TabsContent value="manual" className="space-y-3">
                <PairCodeTab
                  addr={addr}
                  onAddrChange={setAddr}
                  code={code}
                  onCodeChange={setCode}
                  services={services}
                  isConnected={isConnected}
                  pairCode={pairCode}
                  connect={connect}
                />
              </TabsContent>

              <TabsContent value="usb">
                <UsbTab devices={usb} enable={enable} />
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
                  return (
                    <WirelessDeviceRow
                      key={device.serial}
                      device={device}
                      busy={disconnect.isPending && disconnect.variables === target}
                      onDisconnect={() => disconnect.mutate(target)}
                    />
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
