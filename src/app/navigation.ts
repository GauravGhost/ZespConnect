import type { LucideIcon } from "lucide-react";
import {
  AppWindow,
  FolderOpen,
  Gamepad2,
  LayoutDashboard,
  Settings,
  Smartphone,
  Terminal,
  Wifi,
} from "lucide-react";

import type { TabId } from "@/app/store";

export interface NavItem {
  id: TabId;
  label: string;
  icon: LucideIcon;
  /** One-line context shown next to the page title in the header. */
  description: string;
}

export const PRIMARY_NAV: NavItem[] = [
  {
    id: "overview",
    label: "Overview",
    icon: LayoutDashboard,
    description: "Device status and quick settings",
  },
  { id: "apps", label: "Apps", icon: AppWindow, description: "Installed packages" },
  { id: "mirror", label: "Mirror", icon: Smartphone, description: "Screen and camera windows" },
  { id: "wireless", label: "Wireless", icon: Wifi, description: "Pair and connect over Wi-Fi" },
  { id: "files", label: "Files", icon: FolderOpen, description: "Browse device storage" },
  { id: "console", label: "Console", icon: Terminal, description: "Shell, logcat and clipboard" },
  {
    id: "emulators",
    label: "Emulators",
    icon: Gamepad2,
    description: "Android Virtual Devices on this computer",
  },
];

export const SECONDARY_NAV: NavItem[] = [
  { id: "settings", label: "Settings", icon: Settings, description: "Tools and appearance" },
];

const NAV_ITEMS: NavItem[] = [...PRIMARY_NAV, ...SECONDARY_NAV];

export const TAB_IDS: TabId[] = NAV_ITEMS.map((item) => item.id);

export function navItem(tab: TabId): NavItem {
  return NAV_ITEMS.find((item) => item.id === tab) ?? NAV_ITEMS[0];
}
