import { NAV_GROUPS, SETTINGS_NAV, shortcutLabel, type NavItem } from "@/app/navigation";
import { useAppStore } from "@/app/store";
import { IS_MAC } from "@/lib/platform";
import { cn } from "@/lib/utils";

/** Shared with the title bar so the page title lines up with the content edge. */
export const SIDEBAR_WIDTH = "w-[188px]";

/** Sidebar row: icon, label and the keyboard shortcut, with an accent bar on the active page. */
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
      title={item.description}
      className={cn(
        "flex h-8 w-full items-center gap-2.5 rounded-md border-l-2 px-2.5 text-[13px] outline-none transition-colors",
        "focus-visible:ring-2 focus-visible:ring-sidebar-ring",
        active
          ? "border-primary bg-sidebar-accent text-sidebar-accent-foreground font-medium"
          : "text-muted-foreground hover:bg-sidebar-accent/60 hover:text-sidebar-foreground border-transparent",
      )}
    >
      <Icon className="size-4 shrink-0" />
      <span className="truncate">{item.label}</span>
      <kbd className="text-muted-foreground/80 ml-auto font-mono text-[10.5px]">
        {shortcutLabel(item, IS_MAC)}
      </kbd>
    </button>
  );
}

export function Sidebar() {
  return (
    <nav
      aria-label="Main"
      className={cn(
        SIDEBAR_WIDTH,
        "bg-sidebar text-sidebar-foreground flex shrink-0 flex-col border-r border-sidebar-border px-2 pb-2",
      )}
    >
      <div className="flex-1 overflow-y-auto">
        {NAV_GROUPS.map((group) => (
          <section key={group.label} className="flex flex-col gap-0.5">
            <h2 className="text-muted-foreground px-2.5 pt-4 pb-1.5 font-mono text-[10px] font-medium tracking-[0.14em] uppercase">
              {group.label}
            </h2>
            {group.items.map((item) => (
              <NavButton key={item.id} item={item} />
            ))}
          </section>
        ))}
      </div>

      <NavButton item={SETTINGS_NAV} />
    </nav>
  );
}
