import type { LucideIcon } from "lucide-react";
import {
  AppWindow,
  ArrowLeftRight,
  Camera,
  Cable,
  Cpu,
  FolderOpen,
  LaptopMinimal,
  Loader2,
  Monitor,
  MonitorSmartphone,
  Smartphone,
  Terminal,
  Upload,
  Volume2,
  Wifi,
  X,
} from "lucide-react";
import type { ReactNode } from "react";

import { useAppStore } from "@/app/store";
import { PhoneSoundToggle } from "@/components/app/PhoneSoundToggle";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { useClipboardSync, usePhoneAudio } from "@/hooks/useDeviceLinks";
import { useInstallApk } from "@/hooks/useInstallApk";
import { useDeviceInfo } from "@/hooks/useDevices";
import {
  useOpenWindows,
  useStartCamera,
  useStartDesktop,
  useStartMirror,
  useStopOpenWindow,
} from "@/hooks/useMirrors";
import { useToolsStatus } from "@/hooks/useTools";
import { batteryIcon, LOW_BATTERY_PERCENT } from "@/lib/battery";
import { deviceName, hasBatteryLevel, stateLabel, transportLabel } from "@/lib/format";
import { DEFAULT_MIRROR_OPTIONS, KIND_LABEL } from "@/lib/mirror";
import { cn } from "@/lib/utils";
import type { Device } from "@/types";

/** The first Android SDK with desktop windowing on connected displays (Android 16). */
const MIN_DESKTOP_SDK = 36;

function Section({
  title,
  action,
  children,
  className,
}: {
  title: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section
      className={cn(
        "bg-card flex flex-col gap-3 rounded-[1.75rem] border border-border/60 p-4 shadow-sm",
        className,
      )}
    >
      <div className="flex min-h-6 items-center justify-between gap-2">
        <h2 className="text-muted-foreground text-xs font-semibold tracking-wide uppercase">
          {title}
        </h2>
        {action}
      </div>
      {children}
    </section>
  );
}

export function DeviceHero({ device }: { device: Device }) {
  const connected = device.state === "device";
  return (
    <section className="bg-card flex items-center gap-3.5 rounded-[1.75rem] border border-border/60 p-4 shadow-sm">
      <span className="bg-primary-container text-on-primary-container flex size-12 shrink-0 items-center justify-center rounded-2xl">
        <Smartphone className="size-6" />
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-2">
          <h2 className="truncate text-lg leading-tight font-semibold">{deviceName(device)}</h2>
          <Badge variant={connected ? "success" : "warning"} className="shrink-0">
            <span
              className={cn("size-1.5 rounded-full", connected ? "bg-success" : "bg-warning")}
            />
            {stateLabel(device.state)}
          </Badge>
        </div>
        <p className="text-muted-foreground truncate font-mono text-xs" title={device.serial}>
          {device.serial}
        </p>
      </div>
    </section>
  );
}

function StatTile({
  icon: Icon,
  label,
  value,
  detail,
  children,
}: {
  icon: LucideIcon;
  label: string;
  value: string;
  detail?: string;
  children?: ReactNode;
}) {
  return (
    <div className="bg-muted/60 flex min-w-0 flex-col gap-1.5 rounded-[1.25rem] p-3">
      <span className="text-muted-foreground flex items-center gap-1.5 text-xs font-medium">
        <Icon className="size-3.5 shrink-0" />
        {label}
      </span>
      <span className="truncate text-lg leading-none font-semibold tabular-nums">{value}</span>
      {children}
      {detail ? <span className="text-muted-foreground truncate text-xs">{detail}</span> : null}
    </div>
  );
}

export function DeviceStats({
  device,
  battery,
  charging,
  androidVersion,
  sdk,
}: {
  device: Device;
  battery: number | null;
  charging: boolean;
  androidVersion: string | null;
  sdk: string | null;
}) {
  const BatteryIcon = batteryIcon(battery, charging);
  const wifi = device.transport === "wifi";

  return (
    <div className="grid grid-cols-3 gap-2">
      <StatTile
        icon={BatteryIcon}
        label="Battery"
        value={hasBatteryLevel(battery) ? `${battery}%` : "-"}
        detail={charging ? "Charging" : "On battery"}
      >
        {hasBatteryLevel(battery) ? (
          <span className="bg-background/80 h-1.5 overflow-hidden rounded-full">
            <span
              className={cn(
                "block h-full rounded-full transition-all",
                charging
                  ? "bg-success"
                  : battery < LOW_BATTERY_PERCENT
                    ? "bg-destructive"
                    : "bg-primary",
              )}
              style={{ width: `${battery}%` }}
            />
          </span>
        ) : null}
      </StatTile>
      <StatTile
        icon={wifi ? Wifi : Cable}
        label="Connection"
        value={transportLabel(device)}
        detail={wifi ? (device.address ?? "Wireless debugging") : "USB debugging"}
      />
      <StatTile
        icon={Cpu}
        label="System"
        value={androidVersion ? `Android ${androidVersion}` : "-"}
        detail={sdk ? `API level ${sdk}` : undefined}
      />
    </div>
  );
}

function ActionTile({
  icon: Icon,
  label,
  hint,
  onClick,
  disabled,
  busy,
}: {
  icon: LucideIcon;
  label: string;
  hint: string;
  onClick: () => void;
  disabled?: boolean;
  busy?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled || busy}
      title={hint}
      className={cn(
        "group bg-muted/60 hover:bg-muted flex min-w-0 items-center gap-2.5 rounded-[1.25rem] p-2 text-left outline-none",
        "transition-all duration-150 ease-emphasized active:scale-[0.97] focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:pointer-events-none disabled:opacity-50",
      )}
    >
      <span className="bg-background text-primary flex size-8 shrink-0 items-center justify-center rounded-full shadow-xs">
        {busy ? <Loader2 className="size-4 animate-spin" /> : <Icon className="size-4" />}
      </span>
      <span className="min-w-0">
        <span className="block truncate text-sm font-medium">{label}</span>
        <span className="text-muted-foreground block truncate text-xs">{hint}</span>
      </span>
    </button>
  );
}

/** One connected feature: what it does on the left, its control on the right. */
function FeatureRow({
  icon: Icon,
  label,
  hint,
  busy,
  htmlFor,
  children,
}: {
  icon: LucideIcon;
  label: string;
  hint: string;
  busy: boolean;
  htmlFor?: string;
  children: ReactNode;
}) {
  return (
    <div className="bg-muted/60 flex items-center gap-2.5 rounded-[1.25rem] p-2">
      <span className="bg-background text-primary flex size-8 shrink-0 items-center justify-center rounded-full shadow-xs">
        {busy ? <Loader2 className="size-4 animate-spin" /> : <Icon className="size-4" />}
      </span>
      <label htmlFor={htmlFor} className="min-w-0 flex-1">
        <span className="block truncate text-sm font-medium">{label}</span>
        <span className="text-muted-foreground block truncate text-xs">{hint}</span>
      </label>
      {children}
    </div>
  );
}

/** Ongoing links with the phone, each switched on or off per device. */
export function ConnectedFeatures({ device }: { device: Device }) {
  const clipboard = useClipboardSync(device.serial);
  const audio = usePhoneAudio(device.serial);

  return (
    <Section title="Connected features">
      <div className="flex flex-col gap-2">
        <FeatureRow
          icon={ArrowLeftRight}
          label="Clipboard sync"
          hint="Copy on one, paste on the other (text)"
          busy={clipboard.pending}
          htmlFor="feature-clipboard-sync"
        >
          <Switch
            id="feature-clipboard-sync"
            checked={clipboard.on}
            disabled={clipboard.pending}
            onCheckedChange={clipboard.set}
          />
        </FeatureRow>
        <FeatureRow
          icon={Volume2}
          label="Phone sound"
          hint={audio.on ? "The phone's speaker is silent" : "Where the phone's sound plays"}
          busy={audio.pending}
        >
          <PhoneSoundToggle onComputer={audio.on} disabled={audio.pending} onChange={audio.set} />
        </FeatureRow>
      </div>
    </Section>
  );
}

export function QuickActions({ device }: { device: Device }) {
  const serial = device.serial;
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const { data: tools } = useToolsStatus();
  const scrcpyReady = Boolean(tools?.scrcpy.found);
  const startMirror = useStartMirror();
  const startCamera = useStartCamera();
  const startDesktop = useStartDesktop();
  const installApk = useInstallApk();
  const { data: info } = useDeviceInfo(device.state === "device" ? serial : null);
  const desktopSupported = Number(info?.sdk || device.sdk) >= MIN_DESKTOP_SDK;

  return (
    <Section title="Quick actions">
      <div className="grid grid-cols-2 gap-2">
        <ActionTile
          icon={Monitor}
          label="Mirror screen"
          hint={scrcpyReady ? "Open in a window" : "Needs scrcpy"}
          disabled={!scrcpyReady}
          busy={startMirror.isPending}
          onClick={() => startMirror.mutate({ serial, options: DEFAULT_MIRROR_OPTIONS })}
        />
        <ActionTile
          icon={Camera}
          label="Camera"
          hint={scrcpyReady ? "Back camera" : "Needs scrcpy"}
          disabled={!scrcpyReady}
          busy={startCamera.isPending}
          onClick={() => startCamera.mutate({ serial, options: DEFAULT_MIRROR_OPTIONS })}
        />
        {desktopSupported ? (
          <ActionTile
            icon={LaptopMinimal}
            label="Desktop mode"
            hint={scrcpyReady ? "Taskbar & app windows" : "Needs scrcpy"}
            disabled={!scrcpyReady}
            busy={startDesktop.isPending}
            onClick={() => startDesktop.mutate({ serial, label: `${deviceName(device)} desktop` })}
          />
        ) : null}
        <ActionTile
          icon={Upload}
          label="Install APK"
          hint="From your computer"
          busy={installApk.isPending}
          onClick={() => void installApk.pickAndInstall(serial)}
        />
        <ActionTile
          icon={AppWindow}
          label="Apps"
          hint="Launch, uninstall"
          onClick={() => setActiveTab("apps")}
        />
        <ActionTile
          icon={FolderOpen}
          label="Files"
          hint="Browse storage"
          onClick={() => setActiveTab("files")}
        />
        <ActionTile
          icon={Terminal}
          label="Shell"
          hint="adb shell, logcat"
          onClick={() => setActiveTab("console")}
        />
      </div>
    </Section>
  );
}

export function DeviceWindows({ device, className }: { device: Device; className?: string }) {
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const { windows } = useOpenWindows();
  const stopWindow = useStopOpenWindow();
  const mine = windows.filter((window) => window.serial === device.serial);

  return (
    <Section
      title={`Open windows · ${mine.length}`}
      className={className}
      action={
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2.5 text-xs"
          onClick={() => setActiveTab("mirror")}
        >
          Mirror options
        </Button>
      }
    >
      {mine.length === 0 ? (
        <div className="text-muted-foreground flex flex-1 items-center gap-2.5 rounded-[1.25rem] border border-dashed px-3 py-2.5 text-xs">
          <MonitorSmartphone className="size-4 shrink-0" />
          <span>No mirror windows open.</span>
        </div>
      ) : (
        <ul className="-mx-1 flex flex-col gap-1 overflow-y-auto">
          {mine.map((window) => (
            <li
              key={window.key}
              className="hover:bg-muted/60 flex items-center gap-2.5 rounded-2xl px-2 py-1.5"
            >
              <span className="bg-muted text-muted-foreground flex size-8 shrink-0 items-center justify-center rounded-full">
                {window.kind === "camera" ? (
                  <Camera className="size-4" />
                ) : (
                  <Monitor className="size-4" />
                )}
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium">
                  {window.label || KIND_LABEL[window.kind]}
                </span>
                <span className="text-muted-foreground block truncate text-xs">
                  {KIND_LABEL[window.kind]}
                </span>
              </span>
              <Button
                variant="ghost"
                size="icon"
                className="size-8"
                aria-label="Stop window"
                disabled={stopWindow.isPending}
                onClick={() => stopWindow.mutate(window)}
              >
                <X />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}
