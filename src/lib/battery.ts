import {
  BatteryCharging,
  BatteryFull,
  BatteryLow,
  BatteryMedium,
  BatteryWarning,
  type LucideIcon,
} from "lucide-react";

/** The battery glyph Android would show for this level. */
export function batteryIcon(level: number | null | undefined, charging: boolean): LucideIcon {
  if (charging) return BatteryCharging;
  if (typeof level !== "number" || level < 0) return BatteryMedium;
  if (level >= 80) return BatteryFull;
  if (level >= 35) return BatteryMedium;
  if (level >= 15) return BatteryLow;
  return BatteryWarning;
}
