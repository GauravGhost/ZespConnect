import { Smartphone } from "lucide-react";

import { PRIMARY_NAV, SECONDARY_NAV, type NavItem } from "@/app/navigation";
import { useAppStore } from "@/app/store";
import { cn } from "@/lib/utils";

/**
 * Material 3 navigation rail item: a pill indicator behind the icon with the
 * label underneath. Much narrower than a drawer, so content keeps the width.
 */
function RailButton({ item }: { item: NavItem }) {
  const activeTab = useAppStore((s) => s.activeTab);
  const setActiveTab = useAppStore((s) => s.setActiveTab);
  const active = activeTab === item.id;
  const Icon = item.icon;

  return (
    <button
      type="button"
      onClick={() => setActiveTab(item.id)}
      aria-current={active ? "page" : undefined}
      title={item.description}
      className="group flex w-full flex-col items-center gap-1 py-1 outline-none"
    >
      <span
        className={cn(
          "flex h-8 w-14 items-center justify-center rounded-full transition-all duration-150 ease-emphasized group-active:scale-[0.94]",
          "group-focus-visible:ring-2 group-focus-visible:ring-sidebar-ring",
          active
            ? "bg-primary-container text-on-primary-container"
            : "text-muted-foreground group-hover:bg-sidebar-accent group-hover:text-sidebar-accent-foreground",
        )}
      >
        <Icon className="size-[18px]" />
      </span>
      <span
        className={cn(
          "text-[11px] leading-none font-medium transition-colors",
          active ? "text-foreground" : "text-muted-foreground group-hover:text-foreground",
        )}
      >
        {item.label}
      </span>
    </button>
  );
}

export function Sidebar() {
  return (
    <nav
      aria-label="Main"
      className="bg-sidebar text-sidebar-foreground flex w-[76px] shrink-0 flex-col items-center border-r border-sidebar-border"
    >
      <div className="flex h-12 shrink-0 items-center justify-center" title="Zesp Connect">
        <div className="bg-primary-container text-on-primary-container flex size-9 items-center justify-center rounded-2xl shadow-sm">
          <Smartphone className="size-[18px]" />
        </div>
      </div>

      <div className="flex w-full flex-1 flex-col gap-2 overflow-y-auto py-2">
        {PRIMARY_NAV.map((item) => (
          <RailButton key={item.id} item={item} />
        ))}
      </div>

      <div className="flex w-full flex-col gap-2 border-t border-sidebar-border py-2.5">
        {SECONDARY_NAV.map((item) => (
          <RailButton key={item.id} item={item} />
        ))}
      </div>
    </nav>
  );
}
