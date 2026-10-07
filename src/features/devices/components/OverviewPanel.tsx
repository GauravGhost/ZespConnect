import { PlugZap, ShieldAlert, SlidersHorizontal } from "lucide-react";

import { EmptyState } from "@/components/app/EmptyState";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { DeviceIconsLegend, DeviceStatusCard } from "./DeviceStatusCard";
import { useActiveDevice, useDeviceInfo } from "../hooks";

function NoDevice() {
  return (
    <EmptyState
      icon={PlugZap}
      title="No device connected"
      description={
        <>
          Connect an Android device over USB and enable{" "}
          <span className="font-medium">Developer options → USB debugging</span>, then accept the
          authorization prompt on the phone.
        </>
      }
    />
  );
}

export function OverviewPanel() {
  const { device } = useActiveDevice();
  const { data: info } = useDeviceInfo(device?.serial ?? null);

  if (!device) {
    return (
      <div className="flex h-full items-center justify-center p-6">
        <NoDevice />
      </div>
    );
  }

  if (device.state === "unauthorized") {
    return (
      <div className="mx-auto max-w-2xl space-y-4 p-6">
        <Alert variant="warning">
          <ShieldAlert />
          <AlertTitle>Device unauthorized</AlertTitle>
          <AlertDescription>
            Check the prompt on your phone and tap <span className="font-medium">Allow</span> for
            USB debugging. Then reconnect the cable if the device does not appear.
          </AlertDescription>
        </Alert>
        <DeviceStatusCard device={device} info={info} />
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-3xl space-y-6 p-6">
      <DeviceStatusCard device={device} info={info} />

      <div className="rounded-lg border border-dashed p-6 text-center">
        <SlidersHorizontal className="text-muted-foreground mx-auto mb-2 size-5" />
        <p className="text-sm font-medium">Looking for quick controls?</p>
        <p className="text-muted-foreground text-sm">
          Toggles, brightness, volume and reboot are on the Controls tab.
        </p>
      </div>

      <DeviceIconsLegend />
    </div>
  );
}
