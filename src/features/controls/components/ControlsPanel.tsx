import type { LucideIcon } from "lucide-react";
import {
  BellOff,
  Bluetooth,
  Cpu,
  Loader2,
  MapPin,
  Moon,
  Plane,
  PlugZap,
  Power,
  RotateCw,
  ShieldAlert,
  Signal,
  Sun,
  Volume2,
  Wifi,
} from "lucide-react";
import { useEffect, useState } from "react";

import { EmptyState } from "@/components/app/EmptyState";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { useActiveDevice } from "@/features/devices";
import { errorMessage } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { QuickState, QuickToggleKey, RebootMode } from "@/types";
import { useQuickState, useReboot, useSetBrightness, useSetToggle, useSetVolume } from "../hooks";

const TOGGLES: { key: QuickToggleKey; field: keyof QuickState; label: string; icon: LucideIcon }[] =
  [
    { key: "wifi", field: "wifi", label: "Wi-Fi", icon: Wifi },
    { key: "bluetooth", field: "bluetooth", label: "Bluetooth", icon: Bluetooth },
    { key: "darkmode", field: "dark_mode", label: "Dark mode", icon: Moon },
    { key: "airplane", field: "airplane_mode", label: "Airplane mode", icon: Plane },
    { key: "data", field: "mobile_data", label: "Mobile data", icon: Signal },
    { key: "location", field: "location", label: "Location", icon: MapPin },
    { key: "dnd", field: "dnd", label: "Do not disturb", icon: BellOff },
    { key: "rotate", field: "auto_rotate", label: "Auto-rotate", icon: RotateCw },
  ];

const REBOOT_MODES: { mode: RebootMode; label: string; icon: LucideIcon }[] = [
  { mode: "normal", label: "Restart", icon: RotateCw },
  { mode: "bootloader", label: "Bootloader", icon: Cpu },
  { mode: "recovery", label: "Recovery", icon: ShieldAlert },
];

/** A 0–100 slider that keeps a local draft while dragging and commits on release. */
function PercentSlider({
  value,
  disabled,
  onCommit,
}: {
  value: number;
  disabled?: boolean;
  onCommit: (percent: number) => void;
}) {
  const [draft, setDraft] = useState<number | null>(null);

  // Drop the draft once the backend echoes (within a point of) the committed value.
  useEffect(() => {
    if (draft !== null && Math.abs(value - draft) <= 1) setDraft(null);
  }, [value, draft]);

  return (
    <Slider
      value={[draft ?? value]}
      min={0}
      max={100}
      step={1}
      disabled={disabled}
      onValueChange={([next]) => setDraft(next)}
      onValueCommit={([next]) => onCommit(next)}
    />
  );
}

function SliderRow({
  id,
  icon: Icon,
  label,
  value,
  disabled,
  onCommit,
}: {
  id: string;
  icon: LucideIcon;
  label: string;
  value: number;
  disabled?: boolean;
  onCommit: (percent: number) => void;
}) {
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-2">
        <Label htmlFor={id} className="font-normal">
          <Icon className="text-muted-foreground size-4" />
          {label}
        </Label>
        <span className="text-muted-foreground tabular-nums text-xs">{value}%</span>
      </div>
      <PercentSlider value={value} disabled={disabled} onCommit={onCommit} />
    </div>
  );
}

/** A quick-settings tile, styled after the Android notification shade: a
 * rounded squircle with an icon chip and a label, filled in when the setting
 * is on. The whole tile is the control. */
function ToggleTile({
  icon: Icon,
  label,
  checked,
  disabled,
  onChange,
}: {
  icon: LucideIcon;
  label: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (on: boolean) => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "flex h-20 flex-col items-start justify-between rounded-[1.25rem] p-3 text-left outline-none",
        "transition-all duration-150 ease-out active:scale-[0.96] focus-visible:ring-ring focus-visible:ring-offset-background focus-visible:ring-2 focus-visible:ring-offset-2",
        "disabled:pointer-events-none disabled:opacity-60",
        checked
          ? "bg-primary text-primary-foreground hover:bg-primary/90"
          : "bg-muted/60 text-foreground hover:bg-muted",
      )}
    >
      <span
        className={cn(
          "flex size-7 items-center justify-center rounded-full transition-colors",
          checked ? "bg-primary-foreground/20" : "bg-background shadow-xs",
        )}
      >
        <Icon className="size-4" />
      </span>
      <span className="w-full truncate text-xs font-medium">{label}</span>
    </button>
  );
}

export function ControlsPanel() {
  const { device } = useActiveDevice();
  const serial = device?.serial ?? null;
  const { data, isLoading, isError, error } = useQuickState(serial);

  const setToggle = useSetToggle(serial);
  const setBrightness = useSetBrightness(serial);
  const setVolume = useSetVolume(serial);
  const reboot = useReboot(serial);
  const [rebootMode, setRebootMode] = useState<RebootMode | null>(null);

  if (!device) {
    return (
      <div className="flex h-full items-center justify-center p-6">
        <EmptyState
          icon={PlugZap}
          title="No device connected"
          description="Connect an Android device to control its toggles, brightness and volume."
        />
      </div>
    );
  }

  if (isLoading && !data) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="text-muted-foreground size-5 animate-spin" />
      </div>
    );
  }

  if (isError || !data) {
    return (
      <div className="mx-auto max-w-2xl p-6">
        <Alert variant="destructive">
          <AlertTitle>Could not read device state</AlertTitle>
          <AlertDescription>{errorMessage(error)}</AlertDescription>
        </Alert>
      </div>
    );
  }

  const brightnessPct = Math.round((data.brightness / data.brightness_max) * 100);
  const togglesDisabled = setToggle.isPending;

  function confirmReboot() {
    if (!rebootMode) return;
    reboot.mutate(rebootMode, { onSettled: () => setRebootMode(null) });
  }

  return (
    <div className="mx-auto w-full max-w-4xl space-y-4 p-5">
      <div className="flex items-baseline gap-2">
        <h1 className="text-base font-semibold">Controls</h1>
        <p className="text-muted-foreground truncate text-sm">
          {device.display_name || device.model || device.serial}
        </p>
      </div>

      <Card className="gap-4 py-4">
        <CardHeader className="gap-1 px-5">
          <CardTitle className="text-sm">Quick settings</CardTitle>
          <CardDescription>Toggle common system switches on the device.</CardDescription>
        </CardHeader>
        <CardContent className="grid grid-cols-2 gap-2.5 px-5 sm:grid-cols-4">
          {TOGGLES.map(({ key, field, label, icon }) => (
            <ToggleTile
              key={key}
              icon={icon}
              label={label}
              checked={Boolean(data[field])}
              disabled={togglesDisabled}
              onChange={(on) => setToggle.mutate({ key, on })}
            />
          ))}
        </CardContent>
      </Card>

      <Card className="gap-4 py-4">
        <CardHeader className="gap-1 px-5">
          <CardTitle className="text-sm">Display &amp; sound</CardTitle>
          <CardDescription>Adjust the screen brightness and media volume.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-5 px-5">
          <SliderRow
            id="brightness"
            icon={Sun}
            label="Brightness"
            value={brightnessPct}
            disabled={setBrightness.isPending}
            onCommit={(percent) => setBrightness.mutate(percent)}
          />
          <SliderRow
            id="volume"
            icon={Volume2}
            label="Volume"
            value={data.volume}
            disabled={setVolume.isPending}
            onCommit={(percent) => setVolume.mutate(percent)}
          />
        </CardContent>
      </Card>

      <Card className="gap-4 py-4">
        <CardHeader className="gap-1 px-5">
          <CardTitle className="flex items-center gap-2 text-sm">
            <Power className="size-4" />
            Power
          </CardTitle>
          <CardDescription>The device reboots immediately; the connection drops.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-2 px-5">
          {REBOOT_MODES.map(({ mode, label, icon: Icon }) => (
            <Button
              key={mode}
              variant={mode === "normal" ? "outline" : "ghost"}
              size="sm"
              disabled={reboot.isPending}
              onClick={() => setRebootMode(mode)}
            >
              <Icon />
              {label}
            </Button>
          ))}
        </CardContent>
      </Card>

      <Dialog open={rebootMode !== null} onOpenChange={(open) => !open && setRebootMode(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Reboot device?</DialogTitle>
            <DialogDescription>
              {device.display_name || device.serial} will{" "}
              {rebootMode === "normal" ? "restart" : `boot into ${rebootMode} mode`}. The connection
              will be interrupted.
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
    </div>
  );
}
