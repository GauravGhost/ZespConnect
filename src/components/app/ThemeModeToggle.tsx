import { Moon, Sun } from "lucide-react";

import { useAppStore } from "@/app/store";
import { useResolvedTheme } from "@/app/theme/useResolvedTheme";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

/**
 * Flips between Light and Dark. "System" stays available in Settings; when it
 * is active, the button shows the resolved mode and switches to the opposite.
 */
export function ThemeModeToggle() {
  const setThemeMode = useAppStore((state) => state.setThemeMode);
  const resolved = useResolvedTheme();
  const next = resolved === "dark" ? "light" : "dark";
  const Icon = resolved === "dark" ? Moon : Sun;
  const label = `Switch to ${next} mode`;

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="size-9"
          aria-label={label}
          onClick={() => setThemeMode(next)}
        >
          <Icon />
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
