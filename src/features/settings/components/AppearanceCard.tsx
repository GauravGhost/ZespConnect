import { Monitor, Moon, Sun, type LucideIcon } from "lucide-react";

import { useAppStore } from "@/app/store";
import { THEMES } from "@/app/theme/presets";
import type { ThemeMode } from "@/app/theme/types";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";

const MODES: { mode: ThemeMode; label: string; icon: LucideIcon }[] = [
  { mode: "light", label: "Light", icon: Sun },
  { mode: "dark", label: "Dark", icon: Moon },
  { mode: "system", label: "System", icon: Monitor },
];

/** Color-mode + accent-theme picker. Options come from the theme registry. */
export function AppearanceCard() {
  const themeId = useAppStore((state) => state.themeId);
  const setThemeId = useAppStore((state) => state.setThemeId);
  const themeMode = useAppStore((state) => state.themeMode);
  const setThemeMode = useAppStore((state) => state.setThemeMode);

  return (
    <Card>
      <CardHeader className="gap-1">
        <CardTitle className="text-sm">Appearance</CardTitle>
        <CardDescription>Pick a color mode and an accent theme.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="space-y-2.5">
          <p className="text-sm font-medium">Color mode</p>
          <div className="flex flex-wrap gap-2">
            {MODES.map(({ mode, label, icon: Icon }) => {
              const active = themeMode === mode;
              return (
                <button
                  key={mode}
                  type="button"
                  onClick={() => setThemeMode(mode)}
                  aria-pressed={active}
                  className={cn(
                    "inline-flex items-center gap-2 rounded-full px-4 py-2 text-sm font-medium outline-none transition-all duration-150 ease-emphasized active:scale-[0.97] focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                    active
                      ? "bg-primary-container text-on-primary-container shadow-xs"
                      : "bg-muted/60 text-foreground hover:bg-muted",
                  )}
                >
                  <Icon className="size-4" />
                  {label}
                </button>
              );
            })}
          </div>
        </div>

        <div className="space-y-2.5">
          <p className="text-sm font-medium">Accent</p>
          <div className="flex flex-wrap gap-3">
            {THEMES.map((theme) => {
              const active = theme.id === themeId;
              return (
                <button
                  key={theme.id}
                  type="button"
                  onClick={() => setThemeId(theme.id)}
                  aria-pressed={active}
                  aria-label={theme.label}
                  className="flex flex-col items-center gap-1.5 rounded-2xl outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
                >
                  <span
                    className={cn(
                      "flex size-11 items-center justify-center rounded-full shadow-sm transition-all duration-150 ease-emphasized",
                      active
                        ? "ring-2 ring-ring ring-offset-2 ring-offset-background"
                        : "hover:scale-105",
                    )}
                    style={{ backgroundColor: theme.swatch }}
                  />
                  <span className="text-muted-foreground text-xs font-medium">{theme.label}</span>
                </button>
              );
            })}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
