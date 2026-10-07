import type { LucideIcon } from "lucide-react";

import { cn } from "@/lib/utils";

/**
 * Android quick-settings tile: a squircle with an icon chip and a label, filled
 * in when the setting is on. The whole tile is the control. This is the
 * signature Material You element — reuse it instead of re-implementing tiles.
 */
export function QuickTile({
  icon: Icon,
  label,
  checked,
  disabled,
  onChange,
  className,
}: {
  icon: LucideIcon;
  label: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (on: boolean) => void;
  className?: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "flex h-20 flex-col items-start justify-between rounded-[1.25rem] p-3 text-left outline-none",
        "transition-all duration-150 ease-emphasized active:scale-[0.96] focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:pointer-events-none disabled:opacity-60",
        checked
          ? "bg-primary-container text-on-primary-container hover:bg-primary-container/90"
          : "bg-muted/60 text-foreground hover:bg-muted",
        className,
      )}
    >
      <span
        className={cn(
          "flex size-7 items-center justify-center rounded-full transition-colors",
          checked ? "bg-on-primary-container/20" : "bg-background shadow-xs",
        )}
      >
        <Icon className="size-4" />
      </span>
      <span className="w-full truncate text-xs font-medium">{label}</span>
    </button>
  );
}
