import { Check, ChevronDown } from "lucide-react";
import type * as React from "react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

export interface ComboboxOption {
  value: string;
  label?: string;
}

interface ComboboxInputProps extends Omit<React.ComponentProps<"input">, "value" | "onChange"> {
  value: string;
  onValueChange: (value: string) => void;
  options: ComboboxOption[];
  align?: "start" | "end";
}

/** Text input with a dropdown of presets, or type a custom value (resolution, FPS, zoom, …). */
export function ComboboxInput({
  value,
  onValueChange,
  options,
  align = "start",
  className,
  ...props
}: ComboboxInputProps) {
  return (
    <div className="relative">
      <Input
        value={value}
        onChange={(event) => onValueChange(event.target.value)}
        className={cn("pr-8", className)}
        {...props}
      />
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label="Choose a preset"
            className="text-muted-foreground hover:text-foreground absolute top-1/2 right-1 size-7 -translate-y-1/2"
          >
            <ChevronDown className="size-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align={align} className="min-w-[7rem]">
          {options.map((option) => (
            <DropdownMenuItem key={option.value} onSelect={() => onValueChange(option.value)}>
              <span className="flex-1">{option.label ?? option.value}</span>
              {option.value === value ? <Check className="size-4" /> : null}
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
