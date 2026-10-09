import { Copy, CodeXml, Loader2, MoreVertical, Unplug } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { deviceName, stateLabel } from "@/lib/format";
import type { Device } from "@/types";

async function copy(value: string, label: string) {
  try {
    await navigator.clipboard.writeText(value);
    toast.success(`${label} copied`);
  } catch {
    toast.error("Could not copy to clipboard");
  }
}

export function WirelessDeviceRow({
  device,
  busy,
  onDisconnect,
}: {
  device: Device;
  busy: boolean;
  onDisconnect: () => void;
}) {
  return (
    <li className="border-success/40 bg-success/5 rounded-2xl border p-3">
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
            <p className="text-muted-foreground truncate font-mono text-[11px]">{device.address}</p>
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
                void copy(device.serial, "Serial");
              }}
            >
              <Copy />
              Copy serial
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
            <DropdownMenuItem variant="destructive" disabled={busy} onSelect={onDisconnect}>
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
            void copy(device.serial, "Serial");
          }}
        >
          <CodeXml className="size-3.5" />
          Copy serial
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="text-destructive hover:bg-destructive/10 hover:text-destructive"
          disabled={busy}
          onClick={onDisconnect}
        >
          {busy ? <Loader2 className="size-3.5 animate-spin" /> : <Unplug className="size-3.5" />}
          Disconnect
        </Button>
      </div>
    </li>
  );
}
