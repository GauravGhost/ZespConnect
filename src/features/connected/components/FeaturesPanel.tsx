import { PlugZap } from "lucide-react";

import { NoDeviceState } from "@/components/app/EmptyState";
import { useActiveDevice } from "@/hooks/useDevices";
import { ConnectedFeatures } from "./ConnectedFeatures";

export function FeaturesPanel() {
  const { device } = useActiveDevice();

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-3 p-4">
      {device?.state === "device" ? (
        <ConnectedFeatures device={device} />
      ) : (
        <NoDeviceState
          icon={PlugZap}
          description="Connect an Android device to sync its clipboard or play its sound here."
        />
      )}
    </div>
  );
}
