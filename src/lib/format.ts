import type { Device } from "@/types";

export function transportLabel(device: Device): string {
  return device.transport === "wifi" ? "Wi-Fi" : "USB";
}

/** The device's display name, falling back to its model and then its serial. */
export function deviceName(device: Device): string {
  return device.display_name || device.model || device.serial;
}

/** Whether a reported battery level is real (the backend sends -1 or null when unknown). */
export function hasBatteryLevel(level: number | null | undefined): level is number {
  return typeof level === "number" && level >= 0;
}

/** "Pixel 8 · Android 15 · 82% · Wi-Fi" - the header identity line. */
export function deviceIdentity(device: Device): string {
  const parts: string[] = [deviceName(device)];
  if (device.android_version) parts.push(`Android ${device.android_version}`);
  if (hasBatteryLevel(device.battery)) parts.push(`${device.battery}%`);
  parts.push(transportLabel(device));
  return parts.join(" · ");
}

/** "just now", "2 minutes ago", "3 days ago" - for stored timestamps. */
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

const pad2 = (value: number) => String(value).padStart(2, "0");

/** Local time as `yyyy-MM-dd_HH-mm-ss`, safe in file names. */
export function fileStamp(date: Date): string {
  const day = `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}`;
  const time = `${pad2(date.getHours())}-${pad2(date.getMinutes())}-${pad2(date.getSeconds())}`;
  return `${day}_${time}`;
}

/** `m:ss` for a recording timer; minutes keep counting past an hour. */
export function formatElapsed(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  return `${Math.floor(seconds / 60)}:${pad2(seconds % 60)}`;
}
