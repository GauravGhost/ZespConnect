import type { LucideIcon } from "lucide-react";
import { Gamepad2, PlugZap, ShieldAlert, Usb, Wifi } from "lucide-react";
import type { ReactNode } from "react";

import { useAppStore } from "@/app/store";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useActiveDevice, useDeviceInfo } from "@/hooks/useDevices";
import type { Device } from "@/types";
import {
  ConnectedFeatures,
  DeviceHero,
  DeviceStats,
  DeviceWindows,
  QuickActions,
} from "./DeviceSummary";
import { PhoneFrame } from "./PhoneFrame";
import { QuickSettingsShade } from "./QuickSettingsShade";

/** Two-column Overview: the phone on the left, details beside it. */
function OverviewLayout({ phone, children }: { phone: ReactNode; children: ReactNode }) {
  return (
    <div className="mx-auto flex min-h-full w-full max-w-5xl gap-4 p-4">
      {phone}
      <div className="flex min-w-0 flex-1 flex-col gap-3">{children}</div>
    </div>
  );
}

function ScreenMessage({
  icon: Icon,
  title,
  body,
}: {
  icon: LucideIcon;
  title: string;
  body: string;
}) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-4 px-8 text-center">
      <span className="bg-muted text-muted-foreground flex size-16 items-center justify-center rounded-full">
        <Icon className="size-7" />
      </span>
      <div className="space-y-1">
        <p className="text-base font-semibold">{title}</p>
        <p className="text-muted-foreground text-xs">{body}</p>
      </div>
    </div>
  );
}

/** A non-interactive replica of Android's "Allow USB debugging?" prompt. */
function AuthorizePrompt() {
  return (
    <div
      className="bg-foreground/5 flex flex-1 items-center justify-center px-4"
      aria-hidden="true"
    >
      <div className="bg-popover space-y-3 rounded-[1.75rem] p-5 shadow-lg ring-1 ring-border">
        <ShieldAlert className="text-primary size-6" />
        <p className="text-base font-semibold">Allow USB debugging?</p>
        <p className="text-muted-foreground text-xs">
          The computer&apos;s RSA key fingerprint is shown on your phone.
        </p>
        <p className="flex items-center gap-2 text-xs">
          <span className="bg-primary-container size-3.5 rounded-[4px]" />
          Always allow from this computer
        </p>
        <div className="text-primary flex justify-end gap-5 pt-1 text-sm font-semibold">
          <span>Cancel</span>
          <span>Allow</span>
        </div>
      </div>
    </div>
  );
}

function Step({ n, children }: { n: number; children: ReactNode }) {
  return (
    <li className="flex gap-3">
      <span className="bg-primary-container text-on-primary-container flex size-6 shrink-0 items-center justify-center rounded-full text-xs font-semibold">
        {n}
      </span>
      <span className="text-sm leading-6">{children}</span>
    </li>
  );
}

function NoDevice() {
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  return (
    <OverviewLayout
      phone={
        <PhoneFrame>
          <ScreenMessage
            icon={PlugZap}
            title="No device"
            body="Plug in a phone or pair one over Wi-Fi to see it here."
          />
        </PhoneFrame>
      }
    >
      <section className="bg-card space-y-4 rounded-[1.75rem] border border-border/60 p-5 shadow-sm">
        <div>
          <h2 className="text-lg font-semibold">Connect an Android device</h2>
          <p className="text-muted-foreground text-sm">
            Zesp Connect finds devices automatically once debugging is allowed.
          </p>
        </div>
        <ol className="space-y-3">
          <Step n={1}>
            On the phone, open <span className="font-medium">Settings → About phone</span> and tap{" "}
            <span className="font-medium">Build number</span> seven times.
          </Step>
          <Step n={2}>
            Turn on <span className="font-medium">Developer options → USB debugging</span> (or
            Wireless debugging).
          </Step>
          <Step n={3}>
            Connect the cable and tap <span className="font-medium">Allow</span> on the prompt.
          </Step>
        </ol>
      </section>

      <div className="grid grid-cols-3 gap-2">
        <ConnectOption
          icon={Usb}
          title="USB cable"
          body="Detected automatically"
          onClick={() => setActiveTab("wireless")}
        />
        <ConnectOption
          icon={Wifi}
          title="Pair over Wi-Fi"
          body="QR code or pairing code"
          onClick={() => setActiveTab("wireless")}
        />
        <ConnectOption
          icon={Gamepad2}
          title="Emulator"
          body="Start a virtual device"
          onClick={() => setActiveTab("emulators")}
        />
      </div>
    </OverviewLayout>
  );
}

function ConnectOption({
  icon: Icon,
  title,
  body,
  onClick,
}: {
  icon: LucideIcon;
  title: string;
  body: string;
  onClick: () => void;
}) {
  return (
    <Button
      variant="secondary"
      onClick={onClick}
      className="h-auto flex-col items-start gap-2 rounded-[1.25rem] p-3 text-left whitespace-normal"
    >
      <span className="bg-background text-primary flex size-9 items-center justify-center rounded-full">
        <Icon className="size-4" />
      </span>
      <span>
        <span className="block text-sm font-medium">{title}</span>
        <span className="text-muted-foreground block text-xs font-normal">{body}</span>
      </span>
    </Button>
  );
}

export function OverviewPanel() {
  const { device } = useActiveDevice();
  const { data: info } = useDeviceInfo(device?.state === "device" ? device.serial : null);

  if (!device) return <NoDevice />;

  const battery = info?.battery ?? device.battery;
  const charging = info?.charging ?? device.charging ?? false;
  const live: Device = { ...device, battery, charging };
  const stats = (
    <DeviceStats
      device={device}
      battery={battery}
      charging={charging}
      androidVersion={info?.android_version || device.android_version}
      sdk={info?.sdk || device.sdk}
    />
  );

  if (device.state !== "device") {
    const unauthorized = device.state === "unauthorized";
    return (
      <OverviewLayout
        phone={
          <PhoneFrame>
            {unauthorized ? (
              <AuthorizePrompt />
            ) : (
              <ScreenMessage
                icon={PlugZap}
                title="Device offline"
                body="Reconnect the cable or re-pair wireless debugging."
              />
            )}
          </PhoneFrame>
        }
      >
        <DeviceHero device={device} />
        <Alert variant="warning">
          <ShieldAlert />
          <AlertTitle>{unauthorized ? "Waiting for authorization" : "Device not ready"}</AlertTitle>
          <AlertDescription>
            {unauthorized ? (
              <>
                Check the prompt on your phone and tap <span className="font-medium">Allow</span>{" "}
                for USB debugging. Reconnect the cable if the device does not appear.
              </>
            ) : (
              "adb can see this device but can't talk to it yet. It will come back on its own once it's reachable."
            )}
          </AlertDescription>
        </Alert>
        {stats}
      </OverviewLayout>
    );
  }

  return (
    <OverviewLayout phone={<QuickSettingsShade device={live} />}>
      <DeviceHero device={device} />
      {stats}
      <QuickActions device={device} />
      <ConnectedFeatures device={device} />
      <DeviceWindows device={device} className="flex-1" />
    </OverviewLayout>
  );
}
