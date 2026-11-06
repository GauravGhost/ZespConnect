import type { LucideIcon } from "lucide-react";
import {
  AppWindow,
  Blocks,
  FolderOpen,
  Activity,
  Bug,
  Gamepad2,
  Joystick,
  ListTree,
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
  /** Key pressed with Cmd (macOS) or Ctrl to open the tab. */
  shortcutKey: string;
}

export interface NavGroup {
  label: string;
  items: NavItem[];
}

export const NAV_GROUPS: NavGroup[] = [
  {
    label: "Device",
    items: [
      {
        id: "overview",
        label: "Overview",
        icon: LayoutDashboard,
        description: "Device status and quick settings",
        shortcutKey: "1",
      },
      {
        id: "features",
        label: "Features",
        icon: Blocks,
        description: "Connected features, screenshots and recordings",
        shortcutKey: "2",
      },
      {
        id: "apps",
        label: "Apps",
        icon: AppWindow,
        description: "Installed packages",
        shortcutKey: "3",
      },
      {
        id: "mirror",
        label: "Mirror",
        icon: Smartphone,
        description: "Screen and camera windows",
        shortcutKey: "4",
      },
      {
        id: "game",
        label: "Game Mode",
        icon: Joystick,
        description: "Low-latency play and streaming",
        shortcutKey: "g",
      },
      {
        id: "files",
        label: "Files",
        icon: FolderOpen,
        description: "Browse device storage",
        shortcutKey: "5",
      },
    ],
  },
  {
    label: "Tools",
    items: [
      {
        id: "processes",
        label: "Processes",
        icon: ListTree,
        description: "What is running on the device",
        shortcutKey: "6",
      },
      {
        id: "performance",
        label: "Performance",
        icon: Activity,
        description: "Live CPU, memory, FPS, battery and network",
        shortcutKey: "7",
      },
      {
        id: "console",
        label: "Console",
        icon: Terminal,
        description: "Shell, logcat and port forwarding",
        shortcutKey: "8",
      },
      {
        id: "debugging",
        label: "Debugging",
        icon: Bug,
        description: "Developer options, layout inspector and crashes",
        shortcutKey: "9",
      },
      {
        id: "emulators",
        label: "Emulators",
        icon: Gamepad2,
        description: "Android Virtual Devices on this computer",
        shortcutKey: "0",
      },
    ],
  },
];

export const SETTINGS_NAV: NavItem = {
  id: "settings",
  label: "Settings",
  icon: Settings,
  description: "Tools and appearance",
  shortcutKey: ",",
};

/** Pinned to the sidebar's foot: how devices connect, apart from what is done with them. */
export const FOOTER_NAV: NavItem[] = [
  {
    id: "wireless",
    label: "Wireless",
    icon: Wifi,
    description: "Pair and connect over Wi-Fi",
    shortcutKey: ".",
  },
  SETTINGS_NAV,
];

const NAV_ITEMS: NavItem[] = [...NAV_GROUPS.flatMap((group) => group.items), ...FOOTER_NAV];

export const TAB_IDS: TabId[] = NAV_ITEMS.map((item) => item.id);

export function navItem(tab: TabId): NavItem {
  return NAV_ITEMS.find((item) => item.id === tab) ?? NAV_ITEMS[0];
}

type ShortcutPress = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">;

/** The tab a key press opens, or `null` when the press is not a tab shortcut. */
export function tabForShortcut(press: ShortcutPress, isMac: boolean): TabId | null {
  const primary = isMac ? press.metaKey : press.ctrlKey;
  const secondary = isMac ? press.ctrlKey : press.metaKey;
  if (!primary || secondary || press.altKey || press.shiftKey) return null;

  return NAV_ITEMS.find((item) => item.shortcutKey === press.key)?.id ?? null;
}

/** A tab shortcut as the user's keyboard labels it, such as "⌘1" or "Ctrl+1". */
export function shortcutLabel(item: NavItem, isMac: boolean): string {
  return `${isMac ? "⌘" : "Ctrl+"}${item.shortcutKey}`;
}
