import type { Device } from "@/types";

export function formatBytes(size: number): string {
  if (size <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = size;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  return `${value.toFixed(1)} ${units[index]}`;
}

export function transportLabel(device: Device): string {
  return device.transport === "wifi" ? "Wi-Fi" : "USB";
}

/** "Pixel 8 · Android 15 · 82% · Wi-Fi" — the header identity line. */
export function deviceIdentity(device: Device): string {
  const parts: string[] = [device.display_name || device.model || device.serial];
  if (device.android_version) parts.push(`Android ${device.android_version}`);
  if (typeof device.battery === "number" && device.battery >= 0) {
    parts.push(`${device.battery}%`);
  }
  parts.push(transportLabel(device));
  return parts.join(" · ");
}

export function stateLabel(state: Device["state"]): string {
  switch (state) {
    case "device":
      return "Connected";
    case "unauthorized":
      return "Unauthorized";
    case "offline":
      return "Offline";
    case "connecting":
      return "Connecting";
    default:
      return "Unknown";
  }
}
