import { Monitor, Moon, Sun, type LucideIcon } from "lucide-react";

import { useAppStore } from "@/app/store";
import type { ThemeMode } from "@/app/theme/types";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

const ORDER: ThemeMode[] = ["system", "light", "dark"];

const LABEL: Record<ThemeMode, string> = {
  light: "Light",
  dark: "Dark",
  system: "System",
};

const ICON: Record<ThemeMode, LucideIcon> = {
  light: Sun,
  dark: Moon,
  system: Monitor,
};

/** Cycles the app color mode: System -> Light -> Dark. */
export function ThemeModeToggle() {
  const themeMode = useAppStore((state) => state.themeMode);
  const setThemeMode = useAppStore((state) => state.setThemeMode);
  const Icon = ICON[themeMode];

  const cycle = () => {
    const next = ORDER[(ORDER.indexOf(themeMode) + 1) % ORDER.length];
    setThemeMode(next);
  };

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="size-9"
          aria-label={`Theme: ${LABEL[themeMode]}`}
          onClick={cycle}
        >
          <Icon />
        </Button>
      </TooltipTrigger>
      <TooltipContent>Theme: {LABEL[themeMode]}</TooltipContent>
    </Tooltip>
  );
}
