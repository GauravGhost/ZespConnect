import { Moon, Sun } from "lucide-react";

import { useAppStore } from "@/app/store";
import { useResolvedTheme } from "@/app/theme/useResolvedTheme";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

/** Flips between Light and Dark; with "System" active it shows the resolved mode and flips it. */
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
          className="size-8 rounded-lg"
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
