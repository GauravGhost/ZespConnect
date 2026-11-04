import { Monitor, Moon, Sun, type LucideIcon } from "lucide-react";

import { useAppStore } from "@/app/store";
import { THEMES } from "@/app/theme/presets";
import type { ThemeMode } from "@/app/theme/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
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
      <CardHeader>
        <CardTitle className="text-sm">Appearance</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex items-center justify-between gap-4">
          <p className="text-muted-foreground text-sm">Accent</p>
          <div className="flex items-center gap-2">
            {THEMES.map((theme) => {
              const active = theme.id === themeId;
              return (
                <button
                  key={theme.id}
                  type="button"
                  onClick={() => setThemeId(theme.id)}
                  aria-pressed={active}
                  aria-label={theme.label}
                  title={theme.label}
                  className={cn(
                    "size-5 rounded-full outline-none transition-transform duration-150 ease-emphasized focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                    active
                      ? "ring-2 ring-ring ring-offset-2 ring-offset-background"
                      : "hover:scale-110",
                  )}
                  style={{ backgroundColor: theme.swatch }}
                />
              );
            })}
          </div>
        </div>

        <div className="flex items-center justify-between gap-4">
          <p className="text-muted-foreground text-sm">Mode</p>
          <div className="bg-muted/60 inline-flex rounded-full p-0.5">
            {MODES.map(({ mode, label, icon: Icon }) => {
              const active = themeMode === mode;
              return (
                <button
                  key={mode}
                  type="button"
                  onClick={() => setThemeMode(mode)}
                  aria-pressed={active}
                  className={cn(
                    "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium outline-none transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring",
                    active
                      ? "bg-primary-container text-on-primary-container shadow-xs"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  <Icon className="size-3.5" />
                  {label}
                </button>
              );
            })}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
