import type { LucideIcon } from "lucide-react";
import {
  AppWindow,
  FolderOpen,
  Gamepad2,
  LayoutDashboard,
  Settings,
  Smartphone,
  SlidersHorizontal,
  Terminal,
  Wifi,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { useAppStore, type TabId } from "@/app/store";

interface NavItem {
  id: TabId;
  label: string;
  icon: LucideIcon;
}

const PRIMARY: NavItem[] = [
  { id: "overview", label: "Overview", icon: LayoutDashboard },
  { id: "apps", label: "Apps", icon: AppWindow },
  { id: "mirror", label: "Mirror", icon: Smartphone },
  { id: "controls", label: "Controls", icon: SlidersHorizontal },
  { id: "wireless", label: "Wireless", icon: Wifi },
  { id: "files", label: "Files", icon: FolderOpen },
  { id: "console", label: "Console", icon: Terminal },
  { id: "emulators", label: "Emulators", icon: Gamepad2 },
];

const SECONDARY: NavItem[] = [{ id: "settings", label: "Settings", icon: Settings }];

function NavButton({ item }: { item: NavItem }) {
  const activeTab = useAppStore((s) => s.activeTab);
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const active = activeTab === item.id;
  const Icon = item.icon;

  return (
    <button
      type="button"
      onClick={() => setActiveTab(item.id)}
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
        active
          ? "bg-sidebar-accent text-sidebar-accent-foreground"
          : "text-muted-foreground hover:bg-sidebar-accent/60 hover:text-sidebar-accent-foreground",
      )}
    >
      <Icon className="size-4 shrink-0" />
      <span>{item.label}</span>
    </button>
  );
}

export function Sidebar() {
  return (
    <nav className="bg-sidebar text-sidebar-foreground flex w-52 shrink-0 flex-col border-r">
      <div className="flex h-14 items-center gap-2 px-4">
        <div className="bg-primary text-primary-foreground flex size-7 items-center justify-center rounded-md">
          <Smartphone className="size-4" />
        </div>
        <span className="text-sm font-semibold tracking-tight">Zesp Connect</span>
      </div>

      <div className="flex flex-1 flex-col gap-1 overflow-y-auto px-2 py-2">
        {PRIMARY.map((item) => (
          <NavButton key={item.id} item={item} />
        ))}
      </div>

      <div className="flex flex-col gap-1 border-t px-2 py-2">
        {SECONDARY.map((item) => (
          <NavButton key={item.id} item={item} />
        ))}
      </div>
    </nav>
  );
}
