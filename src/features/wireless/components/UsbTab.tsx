import { Loader2, Wifi } from "lucide-react";

import { Button } from "@/components/ui/button";
import { deviceName } from "@/lib/format";
import type { Device } from "@/types";

import type { useEnableTcpip } from "../hooks";

export function UsbTab({
  devices,
  enable,
}: {
  devices: Device[];
  enable: ReturnType<typeof useEnableTcpip>;
}) {
  if (devices.length === 0) {
    return (
      <p className="text-muted-foreground text-sm">
        Connect a phone over USB to switch it to Wi-Fi.
      </p>
    );
  }

  return (
    <ul className="divide-y divide-border/60">
      {devices.map((device) => {
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
              {busy ? <Loader2 className="size-3.5 animate-spin" /> : <Wifi className="size-3.5" />}
              Enable wireless
            </Button>
          </li>
        );
      })}
    </ul>
  );
}
