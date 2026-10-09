export type ThemeMode = "light" | "dark" | "system";

/** Theme tokens keyed by CSS custom-property name, each mapping 1:1 to a `--color-*` in `globals.css`. */
export interface ThemeTokens {
  "--radius": string;
  "--background": string;
  "--foreground": string;
  "--card": string;
  "--card-foreground": string;
  "--popover": string;
  "--popover-foreground": string;
  "--primary": string;
  "--primary-foreground": string;
  /** Fill color for filled controls (buttons, tiles, active tabs, switch). */
  "--primary-container": string;
  /** Text/icon color on `--primary-container`. */
  "--on-primary-container": string;
  "--secondary": string;
  "--secondary-foreground": string;
  "--muted": string;
  "--muted-foreground": string;
  "--accent": string;
  "--accent-foreground": string;
  "--destructive": string;
  "--destructive-foreground": string;
  "--border": string;
  "--input": string;
  "--ring": string;
  "--success": string;
  "--warning": string;
  "--sidebar": string;
  "--sidebar-foreground": string;
  "--sidebar-primary": string;
  "--sidebar-primary-foreground": string;
  "--sidebar-accent": string;
  "--sidebar-accent-foreground": string;
  "--sidebar-border": string;
  "--sidebar-ring": string;
}

export interface ThemePreset {
  id: string;
  label: string;
  /** Color shown in the picker swatch (usually the light `--primary`). */
  swatch: string;
  light: ThemeTokens;
  dark: ThemeTokens;
}
