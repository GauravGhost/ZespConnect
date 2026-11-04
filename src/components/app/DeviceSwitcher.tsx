import { Check, ChevronsUpDown } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { deviceIdentity, deviceName, stateLabel } from "@/lib/format";
import type { Device } from "@/types";

function StatusDot({ state }: { state: Device["state"] }) {
  return (
    <span
      className={cn(
        "size-2 shrink-0 rounded-full",
        state === "device" && "bg-success",
        state === "unauthorized" && "bg-warning",
        state === "offline" && "bg-destructive",
        state === "connecting" && "bg-muted-foreground animate-pulse",
        state === "unknown" && "bg-muted-foreground",
      )}
    />
  );
}

export function DeviceSwitcher({
  device,
  devices,
  onSelect,
}: {
  device: Device | null;
  devices: Device[];
  onSelect: (serial: string) => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          className="h-8 gap-2 rounded-lg border-sidebar-border bg-background/40 pr-2 pl-3 text-[13px]"
          disabled={devices.length === 0}
        >
          {device ? <StatusDot state={device.state} /> : null}
          <span className="max-w-[16rem] truncate">
            {device ? deviceIdentity(device) : "No device"}
          </span>
          <ChevronsUpDown className="text-muted-foreground size-3.5" />
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent align="end" className="w-80">
        <DropdownMenuLabel>Devices</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {devices.map((item) => (
          <DropdownMenuItem
            key={item.serial}
            onSelect={() => onSelect(item.serial)}
            className="items-start gap-2"
          >
            <span className="mt-1">
              <StatusDot state={item.state} />
            </span>
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="truncate font-medium">{deviceName(item)}</span>
              <span className="text-muted-foreground truncate text-xs">
                {item.serial} · {stateLabel(item.state)}
              </span>
            </span>
            {item.serial === device?.serial ? <Check className="mt-0.5 size-4" /> : null}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
