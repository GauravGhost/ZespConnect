import { Laptop, MonitorSmartphone, Smartphone } from "lucide-react";

import { SegmentedChoice } from "@/components/ui/segmented-choice";
import { soundRouteChoices } from "@/lib/phoneSound";
import type { SoundRoute } from "@/types";

const ROUTES = [
  { value: "phone", label: "Phone", icon: Smartphone },
  { value: "computer", label: "Computer", icon: Laptop },
  { value: "both", label: "Both", icon: MonitorSmartphone },
] as const;

/** Segmented Phone / Computer / Both choice for where a device's sound plays. */
export function PhoneSoundToggle({
  route,
  sdk,
  disabled,
  onChange,
}: Readonly<{
  route: SoundRoute;
  sdk: string | null;
  disabled?: boolean;
  onChange: (route: SoundRoute) => void;
}>) {
  return (
    <SegmentedChoice
      label="Play phone sound on"
      options={soundRouteChoices(ROUTES, sdk, route)}
      value={route}
      disabled={disabled}
      onChange={onChange}
    />
  );
}
