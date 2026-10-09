import type { LucideIcon } from "lucide-react";
import {
  AlertTriangle,
  BellOff,
  Bluetooth,
  Cpu,
  Loader2,
  MapPin,
  Moon,
  Plane,
  Power,
  RotateCw,
  ShieldAlert,
  Signal,
  Sun,
  Volume2,
  Wifi,
} from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { QuickTile } from "@/components/ui/quick-tile";
import { Slider } from "@/components/ui/slider";
import { formatClock, useClock } from "@/hooks/useClock";
import { batteryIcon } from "@/lib/battery";
import { deviceName, hasBatteryLevel } from "@/lib/format";
import { errorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { Device, QuickState, QuickToggleKey, RebootMode } from "@/types";
import {
  TOGGLE_FIELD,
  useQuickState,
  useReboot,
  useSetBrightness,
  useSetToggle,
  useSetVolume,
} from "../hooks";
import { PhoneFrame } from "./PhoneFrame";

const TOGGLES: { key: QuickToggleKey; label: string; icon: LucideIcon }[] = [
  { key: "wifi", label: "Wi-Fi", icon: Wifi },
  { key: "data", label: "Mobile data", icon: Signal },
  { key: "bluetooth", label: "Bluetooth", icon: Bluetooth },
  { key: "airplane", label: "Airplane", icon: Plane },
  { key: "darkmode", label: "Dark theme", icon: Moon },
  { key: "dnd", label: "Do not disturb", icon: BellOff },
  { key: "location", label: "Location", icon: MapPin },
  { key: "rotate", label: "Auto-rotate", icon: RotateCw },
];

const REBOOT_MODES: { mode: RebootMode; label: string; icon: LucideIcon }[] = [
  { mode: "normal", label: "Restart", icon: RotateCw },
  { mode: "bootloader", label: "Bootloader", icon: Cpu },
  { mode: "recovery", label: "Recovery", icon: ShieldAlert },
];

/** Shade-style 0-100 bar: keeps a local draft while dragging, commits on release. */
function ShadeSlider({
  label,
  icon: Icon,
  value,
  pending,
  onCommit,
}: {
  label: string;
  icon: LucideIcon;
  value: number;
  pending: boolean;
  onCommit: (percent: number) => void;
}) {
  const [draft, setDraft] = useState<number | null>(null);
  const shown = draft ?? value;

  // Keep the thumb on the drafted value until the write settles, then hand
  // control back to the polled value (the device may quantise it).
  useEffect(() => {
    if (!pending) setDraft(null);
  }, [pending]);

  return (
    <div className="relative">
      <Slider
        size="lg"
        aria-label={label}
        value={[shown]}
        min={0}
        max={100}
        step={1}
        disabled={pending}
        leadingIcon={
          <Icon
            className={cn(
              "transition-colors",
              shown >= 12 ? "text-primary-foreground" : "text-muted-foreground",
            )}
          />
        }
        onValueChange={([next]) => setDraft(next)}
        onValueCommit={([next]) => onCommit(next)}
      />
      <span
        className={cn(
          "pointer-events-none absolute top-1/2 right-3.5 -translate-y-1/2 text-[11px] font-semibold tabular-nums",
          shown >= 88 ? "text-primary-foreground" : "text-muted-foreground",
        )}
      >
        {shown}%
      </span>
    </div>
  );
}

function StatusIcons({ quick, device }: { quick: QuickState | undefined; device: Device }) {
  const charging = device.charging ?? false;
  const Battery = batteryIcon(device.battery, charging);
  return (
    <>
      {quick?.dnd ? <BellOff /> : null}
      {quick?.bluetooth ? <Bluetooth /> : null}
      {quick?.airplane_mode ? <Plane /> : null}
      {quick?.wifi ? <Wifi /> : null}
      {quick?.mobile_data && !quick.airplane_mode ? <Signal /> : null}
      <Battery className={cn(charging && "text-success")} />
      {hasBatteryLevel(device.battery) ? (
        <span className="tabular-nums">{device.battery}%</span>
      ) : null}
    </>
  );
}

function ShadeMessage({
  icon: Icon,
  title,
  body,
}: {
  icon: LucideIcon;
  title: string;
  body?: string;
}) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3 px-8 text-center">
      <span className="bg-muted text-muted-foreground flex size-12 items-center justify-center rounded-full">
        <Icon className={cn("size-5", Icon === Loader2 && "animate-spin")} />
      </span>
      <div className="space-y-1">
        <p className="text-sm font-medium">{title}</p>
        {body ? <p className="text-muted-foreground text-xs break-words">{body}</p> : null}
      </div>
    </div>
  );
}

/** The Overview phone mockup, rendered as the Android quick-settings shade; tiles drive the real device. */
export function QuickSettingsShade({ device }: { device: Device }) {
  const serial = device.serial;
  const now = useClock();
  const { data, isLoading, isError, error } = useQuickState(serial);

  const setToggle = useSetToggle(serial);
  const setBrightness = useSetBrightness(serial);
  const setVolume = useSetVolume(serial);
  const reboot = useReboot(serial);
  const [rebootMode, setRebootMode] = useState<RebootMode | null>(null);

  const name = deviceName(device);

  function confirmReboot() {
    if (!rebootMode) return;
    reboot.mutate(rebootMode, { onSettled: () => setRebootMode(null) });
  }

  return (
    <PhoneFrame statusLeft={name} status={<StatusIcons quick={data} device={device} />}>
      <div className="flex items-end justify-between px-5 pt-2 pb-3">
        <div>
          <p className="text-[40px] leading-none font-light tracking-tight tabular-nums">
            {formatClock(now)}
          </p>
          <p className="text-muted-foreground mt-1 text-xs font-medium">
            {now.toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" })}
          </p>
        </div>
        {device.android_version ? (
          <span className="bg-muted/80 text-muted-foreground rounded-full px-2.5 py-1 text-[11px] font-medium">
            Android {device.android_version}
          </span>
        ) : null}
      </div>

      {isLoading && !data ? (
        <ShadeMessage icon={Loader2} title="Reading quick settings…" />
      ) : isError || !data ? (
        <ShadeMessage
          icon={AlertTriangle}
          title="Couldn't read device state"
          body={errorMessage(error)}
        />
      ) : (
        <div className="flex flex-1 flex-col gap-2.5 px-3">
          <ShadeSlider
            label="Brightness"
            icon={Sun}
            value={Math.round((data.brightness / Math.max(1, data.brightness_max)) * 100)}
            pending={setBrightness.isPending}
            onCommit={(percent) => setBrightness.mutate(percent)}
          />

          <div className="grid grid-cols-2 gap-2">
            {TOGGLES.map(({ key, label, icon }) => (
              <QuickTile
                key={key}
                layout="wide"
                icon={icon}
                label={label}
                checked={data[TOGGLE_FIELD[key]]}
                disabled={setToggle.isPending}
                onChange={(on) => setToggle.mutate({ key, on })}
              />
            ))}
          </div>

          <ShadeSlider
            label="Media volume"
            icon={Volume2}
            value={data.volume}
            pending={setVolume.isPending}
            onCommit={(percent) => setVolume.mutate(percent)}
          />
        </div>
      )}

      <div className="flex items-center justify-end gap-2 px-5 pt-2 pb-1">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="secondary"
              size="icon"
              className="size-8 shrink-0"
              aria-label="Power menu"
              title="Power menu"
              disabled={reboot.isPending}
            >
              <Power className="size-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" side="top" className="w-44">
            <DropdownMenuLabel>Power</DropdownMenuLabel>
            <DropdownMenuSeparator />
            {REBOOT_MODES.map(({ mode, label, icon: Icon }) => (
              <DropdownMenuItem key={mode} onSelect={() => setRebootMode(mode)}>
                <Icon />
                {label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      <Dialog open={rebootMode !== null} onOpenChange={(open) => !open && setRebootMode(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Reboot device?</DialogTitle>
            <DialogDescription>
              {name} will {rebootMode === "normal" ? "restart" : `boot into ${rebootMode} mode`}.
              The connection will be interrupted.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRebootMode(null)}>
              Cancel
            </Button>
            <Button variant="destructive" disabled={reboot.isPending} onClick={confirmReboot}>
              {reboot.isPending ? <Loader2 className="animate-spin" /> : <Power />}
              Reboot
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </PhoneFrame>
  );
}
