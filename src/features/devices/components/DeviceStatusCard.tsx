import { Battery, BatteryCharging, Cpu, Smartphone, Wifi } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { transportLabel } from "@/lib/format";
import type { Device, DeviceInfo } from "@/types";

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-muted-foreground text-xs">{label}</span>
      <span className="text-sm font-medium">{value}</span>
    </div>
  );
}

export function DeviceStatusCard({
  device,
  info,
}: {
  device: Device;
  info: DeviceInfo | undefined;
}) {
  const battery = info?.battery ?? device.battery;
  const charging = info?.charging ?? device.charging ?? false;

  return (
    <Card className="min-w-0">
      <CardHeader className="min-w-0">
        <div className="flex items-center justify-between gap-2">
          <CardTitle className="flex min-w-0 items-center gap-2">
            <Smartphone className="size-4 shrink-0" />
            <span className="truncate">{device.display_name || device.model}</span>
          </CardTitle>
          {device.state === "device" ? (
            <Badge variant="success" className="shrink-0">
              Connected
            </Badge>
          ) : (
            <Badge variant="warning" className="shrink-0">
              {device.state}
            </Badge>
          )}
        </div>
        <CardDescription className="font-mono text-xs break-all">{device.serial}</CardDescription>
      </CardHeader>

      <CardContent className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <Stat label="Android" value={info?.android_version || device.android_version || "—"} />
        <Stat label="SDK" value={info?.sdk || device.sdk || "—"} />
        <Stat
          label="Battery"
          value={
            typeof battery === "number" && battery >= 0
              ? `${battery}%${charging ? " ⚡" : ""}`
              : "—"
          }
        />
        <Stat label="Transport" value={transportLabel(device)} />
      </CardContent>
    </Card>
  );
}

export function DeviceIconsLegend() {
  return (
    <div className="text-muted-foreground flex flex-wrap items-center gap-4 text-xs">
      <span className="inline-flex items-center gap-1">
        <Wifi className="size-3.5" /> Wi-Fi / USB transport
      </span>
      <span className="inline-flex items-center gap-1">
        <Cpu className="size-3.5" /> SDK level
      </span>
      <span className="inline-flex items-center gap-1">
        <Battery className="size-3.5" />
        <BatteryCharging className="size-3.5" /> Battery / charging
      </span>
    </div>
  );
}
