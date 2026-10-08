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

/** "just now", "2 minutes ago", "3 days ago" — for stored timestamps. */
export function timeAgo(timestamp: number): string {
  const seconds = Math.max(0, Math.round((Date.now() - timestamp) / 1000));
  if (seconds < 45) return "just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} minute${minutes === 1 ? "" : "s"} ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} hour${hours === 1 ? "" : "s"} ago`;
  const days = Math.round(hours / 24);
  if (days < 30) return `${days} day${days === 1 ? "" : "s"} ago`;
  const months = Math.round(days / 30);
  if (months < 12) return `${months} month${months === 1 ? "" : "s"} ago`;
  const years = Math.round(months / 12);
  return `${years} year${years === 1 ? "" : "s"} ago`;
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
