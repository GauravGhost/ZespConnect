import type * as React from "react";

import { cn } from "@/lib/utils";

function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      type={type}
      data-slot="input"
      className={cn(
        "file:text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground flex h-10 w-full min-w-0 rounded-2xl border border-transparent bg-muted/40 px-4 py-1 text-base transition-[color,box-shadow,background-color] duration-150 ease-emphasized outline-none file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
        "focus-visible:bg-muted focus-visible:ring-2 focus-visible:ring-ring",
        "aria-invalid:ring-destructive/30 aria-invalid:ring-2",
        className,
      )}
      {...props}
    />
  );
}

export { Input };
