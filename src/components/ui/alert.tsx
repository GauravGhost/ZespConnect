import { AlertTriangle } from "lucide-react";
import type * as React from "react";

import { cn } from "@/lib/utils";

type AlertVariant = "default" | "warning" | "destructive";

const variantClasses: Record<AlertVariant, string> = {
  default: "bg-card text-card-foreground",
  warning: "bg-warning/10 text-foreground border-warning/30",
  destructive: "bg-destructive/10 text-foreground border-destructive/30",
};

function Alert({
  className,
  variant = "default",
  ...props
}: React.ComponentProps<"div"> & { variant?: AlertVariant }) {
  return (
    <div
      role="alert"
      data-slot="alert"
      className={cn(
        "relative grid w-full grid-cols-[auto_1fr] items-start gap-x-3 gap-y-1 rounded-lg border px-4 py-3 text-sm [&>svg]:size-4 [&>svg]:translate-y-0.5",
        variantClasses[variant],
        className,
      )}
      {...props}
    />
  );
}

function AlertTitle({ className, ...props }: React.ComponentProps<"div">) {
  return <div className={cn("col-start-2 font-medium", className)} {...props} />;
}

function AlertDescription({ className, ...props }: React.ComponentProps<"div">) {
  return <div className={cn("text-muted-foreground col-start-2 text-sm", className)} {...props} />;
}

export { Alert, AlertTitle, AlertDescription, AlertTriangle };
