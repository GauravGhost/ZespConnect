import * as SliderPrimitive from "@radix-ui/react-slider";
import type * as React from "react";

import { cn } from "@/lib/utils";

/**
 * Android 17 (Material 3 expressive) style slider: a thick rounded track with a
 * solid fill and a tall, thin pill-shaped handle. An optional `leadingIcon` sits
 * inside the track on the left, like the brightness/volume controls in the
 * Quick Settings shade.
 */
function Slider({
  className,
  defaultValue,
  value,
  min = 0,
  max = 100,
  leadingIcon,
  ...props
}: React.ComponentProps<typeof SliderPrimitive.Root> & { leadingIcon?: React.ReactNode }) {
  const thumbCount = Array.isArray(value)
    ? value.length
    : Array.isArray(defaultValue)
      ? defaultValue.length
      : 1;

  return (
    <SliderPrimitive.Root
      data-slot="slider"
      defaultValue={defaultValue}
      value={value}
      min={min}
      max={max}
      className={cn(
        "relative flex w-full touch-none items-center select-none data-[disabled]:opacity-50 data-[orientation=vertical]:h-full data-[orientation=vertical]:min-h-44 data-[orientation=vertical]:w-auto data-[orientation=vertical]:flex-col",
        className,
      )}
      {...props}
    >
      <SliderPrimitive.Track
        data-slot="slider-track"
        className={cn(
          "bg-muted relative grow overflow-hidden rounded-full",
          "data-[orientation=horizontal]:h-4 data-[orientation=horizontal]:w-full",
          "data-[orientation=vertical]:h-full data-[orientation=vertical]:w-4",
        )}
      >
        <SliderPrimitive.Range
          data-slot="slider-range"
          className="bg-primary absolute data-[orientation=horizontal]:h-full data-[orientation=vertical]:w-full"
        />
      </SliderPrimitive.Track>

      {leadingIcon ? (
        <span
          data-slot="slider-icon"
          aria-hidden="true"
          className="pointer-events-none absolute top-1/2 left-2 z-10 flex -translate-y-1/2 items-center justify-center [&_svg]:size-4"
        >
          {leadingIcon}
        </span>
      ) : null}

      {Array.from({ length: thumbCount }, (_, index) => (
        <SliderPrimitive.Thumb
          data-slot="slider-thumb"
          key={index}
          className={cn(
            "bg-primary block rounded-full shadow-sm outline-none",
            "ring-ring focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-background",
            "data-[orientation=horizontal]:h-9 data-[orientation=horizontal]:w-2",
            "data-[orientation=vertical]:h-2 data-[orientation=vertical]:w-9",
            "disabled:pointer-events-none",
          )}
        />
      ))}
    </SliderPrimitive.Root>
  );
}

export { Slider };
