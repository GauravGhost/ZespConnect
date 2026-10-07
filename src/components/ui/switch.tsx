import * as SwitchPrimitive from "@radix-ui/react-switch";
import { Check } from "lucide-react";
import type * as React from "react";

import { cn } from "@/lib/utils";

/** Material 3 switch: a pill track whose thumb grows and shows a check when on. */
function Switch({ className, ...props }: React.ComponentProps<typeof SwitchPrimitive.Root>) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cn(
        "peer inline-flex h-7 w-12 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors duration-150 ease-emphasized outline-none",
        "data-[state=checked]:bg-primary-container data-[state=unchecked]:border-border data-[state=unchecked]:bg-muted",
        "focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className={cn(
          "group pointer-events-none flex items-center justify-center rounded-full bg-background shadow-sm transition-all duration-150 ease-emphasized",
          "size-5 data-[state=unchecked]:translate-x-0.5",
          "data-[state=checked]:size-6 data-[state=checked]:translate-x-[1.125rem] data-[state=checked]:bg-on-primary-container",
        )}
      >
        <Check className="text-primary-container size-3.5 opacity-0 transition-opacity duration-150 group-data-[state=checked]:opacity-100" />
      </SwitchPrimitive.Thumb>
    </SwitchPrimitive.Root>
  );
}

export { Switch };
