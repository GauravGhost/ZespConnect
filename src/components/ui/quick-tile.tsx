import type { LucideIcon } from "lucide-react";

import { cn } from "@/lib/utils";

/**
 * Android quick-settings tile: a squircle with an icon chip and a label, filled
 * in when the setting is on. The whole tile is the control. This is the
 * signature Material You element — reuse it instead of re-implementing tiles.
 *
 * `layout="wide"` is the shade's horizontal pill (icon left, label and On/Off
 * underneath), used inside the phone mockup on Overview.
 */
export function QuickTile({
  icon: Icon,
  label,
  checked,
  disabled,
  onChange,
  layout = "square",
  className,
}: {
  icon: LucideIcon;
  label: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (on: boolean) => void;
  layout?: "square" | "wide";
  className?: string;
}) {
  const wide = layout === "wide";

  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "flex text-left outline-none",
        wide
          ? "h-[52px] min-w-0 items-center gap-2.5 rounded-[1.25rem] px-2.5"
          : "h-20 flex-col items-start justify-between rounded-[1.25rem] p-3",
        "transition-all duration-150 ease-emphasized active:scale-[0.96] focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:pointer-events-none disabled:opacity-60",
        checked
          ? "bg-primary-container text-on-primary-container hover:bg-primary-container/90"
          : "bg-muted/70 text-foreground hover:bg-muted",
        className,
      )}
    >
      <span
        className={cn(
          "flex shrink-0 items-center justify-center rounded-full transition-colors",
          wide ? "size-8" : "size-7",
          checked ? "bg-on-primary-container/20" : "bg-background shadow-xs",
        )}
      >
        <Icon className="size-4" />
      </span>
      {wide ? (
        <span className="flex min-w-0 flex-col">
          <span className="line-clamp-2 text-xs leading-tight font-semibold">{label}</span>
          <span
            className={cn(
              "text-[11px] leading-tight",
              checked ? "text-on-primary-container/75" : "text-muted-foreground",
            )}
          >
            {checked ? "On" : "Off"}
          </span>
        </span>
      ) : (
        <span className="w-full truncate text-xs font-medium">{label}</span>
      )}
    </button>
  );
}
