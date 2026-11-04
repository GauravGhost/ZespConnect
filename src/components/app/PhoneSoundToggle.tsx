import { Laptop, Smartphone } from "lucide-react";

import { cn } from "@/lib/utils";

const ROUTES = [
  { onComputer: false, label: "Phone", icon: Smartphone },
  { onComputer: true, label: "Computer", icon: Laptop },
] as const;

/** Segmented Phone / Computer choice for where a device's sound plays. */
export function PhoneSoundToggle({
  onComputer,
  disabled,
  onChange,
}: Readonly<{
  onComputer: boolean;
  disabled?: boolean;
  onChange: (onComputer: boolean) => void;
}>) {
  return (
    <div
      role="radiogroup"
      aria-label="Play phone sound on"
      className="bg-muted/60 flex shrink-0 rounded-full p-1"
    >
      {ROUTES.map(({ onComputer: value, label, icon: Icon }) => {
        const selected = onComputer === value;
        return (
          <button
            key={label}
            type="button"
            role="radio"
            aria-checked={selected}
            disabled={disabled}
            onClick={() => {
              if (!selected) onChange(value);
            }}
            className={cn(
              "flex items-center gap-1.5 rounded-full px-3 py-1.5 text-xs font-medium transition-colors outline-none",
              "focus-visible:ring-ring focus-visible:ring-2 disabled:opacity-60",
              selected
                ? "bg-primary-container text-on-primary-container shadow-sm"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            <Icon className="size-3.5" />
            {label}
          </button>
        );
      })}
    </div>
  );
}
