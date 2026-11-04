import type { LucideIcon } from "lucide-react";

import { cn } from "@/lib/utils";

export interface SegmentOption<T extends string> {
  value: T;
  label: string;
  icon: LucideIcon;
}

/** A pill-shaped single choice between a few options. */
export function SegmentedChoice<T extends string>({
  label,
  options,
  value,
  disabled,
  onChange,
}: Readonly<{
  label: string;
  options: readonly SegmentOption<T>[];
  value: T;
  disabled?: boolean;
  onChange: (value: T) => void;
}>) {
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className="bg-muted/60 flex shrink-0 rounded-full p-1"
    >
      {options.map(({ value: option, label: text, icon: Icon }) => {
        const selected = value === option;
        return (
          <button
            key={option}
            type="button"
            role="radio"
            aria-checked={selected}
            disabled={disabled}
            onClick={() => {
              if (!selected) onChange(option);
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
            {text}
          </button>
        );
      })}
    </div>
  );
}
