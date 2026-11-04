import type { LucideIcon } from "lucide-react";
import type { ButtonHTMLAttributes } from "react";

import { cn } from "@/lib/utils";

type MirrorIconButtonProps = Readonly<
  Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "title"> & {
    label: string;
    icon: LucideIcon;
    /** A toggle that is on fills with the accent container, like an Android quick-settings tile. */
    pressed?: boolean;
    /** Close turns red on hover, as a destructive action. */
    danger?: boolean;
    iconClassName?: string;
  }
>;

/** Material 3 icon button shared by a mirror window's title bar and control bar. */
export function MirrorIconButton({
  label,
  icon: Icon,
  pressed,
  danger,
  className,
  iconClassName,
  ...props
}: MirrorIconButtonProps) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      aria-pressed={pressed}
      className={cn(
        "flex size-6 shrink-0 items-center justify-center rounded-full outline-none",
        "transition-all duration-150 ease-emphasized active:scale-90",
        "focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-60",
        pressed
          ? "bg-primary-container text-on-primary-container hover:bg-primary-container/85"
          : "text-muted-foreground hover:bg-foreground/10 hover:text-foreground",
        danger && "hover:bg-destructive hover:text-white",
        className,
      )}
      {...props}
    >
      <Icon className={cn("size-3.5", iconClassName)} />
    </button>
  );
}
