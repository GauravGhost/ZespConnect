import type { LucideIcon } from "lucide-react";

import { cn } from "@/lib/utils";

/** A compact on/off pill for option lists where a full quick tile is too heavy. */
export function ToggleChip({
  icon: Icon,
  label,
  checked,
  disabled,
  onChange,
}: Readonly<{
  icon: LucideIcon;
  label: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (on: boolean) => void;
}>) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "flex h-8 items-center gap-1.5 rounded-full border px-3 text-xs font-medium outline-none",
        "transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60",
        checked
          ? "bg-primary-container text-on-primary-container border-transparent"
          : "text-muted-foreground hover:text-foreground border-border/60 hover:bg-muted/60",
      )}
    >
      <Icon className="size-3.5" />
      {label}
    </button>
  );
}
