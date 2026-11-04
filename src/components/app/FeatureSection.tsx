import type { LucideIcon } from "lucide-react";
import { Loader2 } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export function Section({
  title,
  action,
  children,
  className,
}: {
  title: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section
      className={cn(
        "bg-card flex flex-col gap-3 rounded-[1.75rem] border border-border/60 p-4 shadow-sm",
        className,
      )}
    >
      <div className="flex min-h-6 items-center justify-between gap-2">
        <h2 className="text-muted-foreground text-xs font-semibold tracking-wide uppercase">
          {title}
        </h2>
        {action}
      </div>
      {children}
    </section>
  );
}

export function FeatureRow({
  icon: Icon,
  label,
  hint,
  busy,
  htmlFor,
  children,
}: {
  icon: LucideIcon;
  label: string;
  hint: string;
  busy: boolean;
  htmlFor?: string;
  children: ReactNode;
}) {
  return (
    <div className="bg-muted/60 flex items-center gap-2.5 rounded-[1.25rem] p-2">
      <span className="bg-background text-primary flex size-8 shrink-0 items-center justify-center rounded-full shadow-xs">
        {busy ? <Loader2 className="size-4 animate-spin" /> : <Icon className="size-4" />}
      </span>
      <label htmlFor={htmlFor} className="min-w-0 flex-1">
        <span className="block truncate text-sm font-medium">{label}</span>
        <span className="text-muted-foreground block truncate text-xs">{hint}</span>
      </label>
      {children}
    </div>
  );
}
