import { Laptop, Smartphone } from "lucide-react";

import { SegmentedChoice } from "@/components/ui/segmented-choice";

const ROUTES = [
  { value: "phone", label: "Phone", icon: Smartphone },
  { value: "computer", label: "Computer", icon: Laptop },
] as const;

/** Segmented Phone / Computer choice for where a device's sound plays. */
export function PhoneSoundToggle({
  onComputer,
  disabled,
  onChange,
}: Readonly<{
  onComputer: boolean;
  disabled?: boolean;
  onChange: (onComputer: boolean) => void;
}>) {
  return (
    <SegmentedChoice
      label="Play phone sound on"
      options={ROUTES}
      value={onComputer ? "computer" : "phone"}
      disabled={disabled}
      onChange={(route) => onChange(route === "computer")}
    />
  );
}
