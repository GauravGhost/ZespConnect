import { LayoutGrid, List, type LucideIcon } from "lucide-react";

import type { AppsView } from "@/app/store";
import { cn } from "@/lib/utils";

const VIEWS: { id: AppsView; label: string; icon: LucideIcon }[] = [
  { id: "list", label: "List view", icon: List },
  { id: "grid", label: "Grid view", icon: LayoutGrid },
];

export function AppsViewToggle({
  value,
  onChange,
}: {
  value: AppsView;
  onChange: (view: AppsView) => void;
}) {
  return (
    <div
      role="radiogroup"
      aria-label="App layout"
      className="bg-muted flex shrink-0 items-center gap-0.5 rounded-full p-0.5"
    >
      {VIEWS.map(({ id, label, icon: Icon }) => (
        <button
          key={id}
          type="button"
          role="radio"
          aria-checked={value === id}
          aria-label={label}
          title={label}
          onClick={() => onChange(id)}
          className={cn(
            "flex size-8 items-center justify-center rounded-full transition-all duration-150 ease-emphasized",
            value === id
              ? "bg-primary-container text-on-primary-container shadow-xs"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          <Icon className="size-4" />
        </button>
      ))}
    </div>
  );
}
