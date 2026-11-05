import type { ThemePreset, ThemeTokens } from "./types";

type SurfaceTokens = Omit<
  ThemeTokens,
  "--primary" | "--primary-foreground" | "--ring" | "--primary-container" | "--on-primary-container"
>;

const NEUTRAL_LIGHT: SurfaceTokens = {
  "--radius": "1rem",
  "--background": "oklch(0.985 0 0)",
  "--foreground": "oklch(0.16 0 0)",
  "--card": "oklch(0.995 0 0)",
  "--card-foreground": "oklch(0.16 0 0)",
  "--popover": "oklch(0.995 0 0)",
  "--popover-foreground": "oklch(0.16 0 0)",
  "--secondary": "oklch(0.95 0.005 155)",
  "--secondary-foreground": "oklch(0.22 0 0)",
  "--muted": "oklch(0.955 0 0)",
  "--muted-foreground": "oklch(0.5 0 0)",
  "--accent": "oklch(0.94 0.02 155)",
  "--accent-foreground": "oklch(0.22 0 0)",
  "--destructive": "oklch(0.577 0.245 27.325)",
  "--destructive-foreground": "oklch(0.985 0 0)",
  "--border": "oklch(0.9 0 0)",
  "--input": "oklch(0.92 0 0)",
  "--success": "oklch(0.62 0.17 150)",
  "--warning": "oklch(0.75 0.16 75)",
  "--sidebar": "oklch(0.97 0.006 155)",
  "--sidebar-foreground": "oklch(0.16 0 0)",
  "--sidebar-primary": "oklch(0.4 0.12 155)",
  "--sidebar-primary-foreground": "oklch(0.985 0 0)",
  "--sidebar-accent": "oklch(0.91 0.03 155)",
  "--sidebar-accent-foreground": "oklch(0.2 0 0)",
  "--sidebar-border": "oklch(0.9 0 0)",
  "--sidebar-ring": "oklch(0.7 0 0)",
};

const NEUTRAL_DARK: SurfaceTokens = {
  "--radius": "1rem",
  "--background": "oklch(0.145 0 0)",
  "--foreground": "oklch(0.97 0 0)",
  "--card": "oklch(0.205 0 0)",
  "--card-foreground": "oklch(0.97 0 0)",
  "--popover": "oklch(0.23 0 0)",
  "--popover-foreground": "oklch(0.97 0 0)",
  "--secondary": "oklch(0.27 0.01 155)",
  "--secondary-foreground": "oklch(0.97 0 0)",
  "--muted": "oklch(0.26 0 0)",
  "--muted-foreground": "oklch(0.72 0 0)",
  "--accent": "oklch(0.3 0.03 155)",
  "--accent-foreground": "oklch(0.97 0 0)",
  "--destructive": "oklch(0.704 0.191 22.216)",
  "--destructive-foreground": "oklch(0.985 0 0)",
  "--border": "oklch(1 0 0 / 12%)",
  "--input": "oklch(1 0 0 / 18%)",
  "--success": "oklch(0.7 0.15 150)",
  "--warning": "oklch(0.8 0.15 75)",
  "--sidebar": "oklch(0.185 0 0)",
  "--sidebar-foreground": "oklch(0.97 0 0)",
  "--sidebar-primary": "oklch(0.7 0.16 155)",
  "--sidebar-primary-foreground": "oklch(0.15 0 0)",
  "--sidebar-accent": "oklch(0.28 0.03 155)",
  "--sidebar-accent-foreground": "oklch(0.97 0 0)",
  "--sidebar-border": "oklch(1 0 0 / 12%)",
  "--sidebar-ring": "oklch(0.556 0 0)",
};

interface Accent {
  primary: string;
  primaryForeground: string;
  ring: string;
  darkPrimary: string;
  darkPrimaryForeground: string;
  darkRing: string;
  /** Deep, muted accent used as the fill for controls in dark mode. */
  darkPrimaryContainer: string;
  darkOnPrimaryContainer: string;
}

/** Compose a preset from the shared neutrals + accent; the container darkens in dark mode for readable text. */
function defineTheme(id: string, label: string, accent: Accent): ThemePreset {
  return {
    id,
    label,
    swatch: accent.primary,
    light: {
      ...NEUTRAL_LIGHT,
      "--primary": accent.primary,
      "--primary-foreground": accent.primaryForeground,
      "--ring": accent.ring,
      "--primary-container": accent.primary,
      "--on-primary-container": accent.primaryForeground,
    },
    dark: {
      ...NEUTRAL_DARK,
      "--primary": accent.darkPrimary,
      "--primary-foreground": accent.darkPrimaryForeground,
      "--ring": accent.darkRing,
      "--primary-container": accent.darkPrimaryContainer,
      "--on-primary-container": accent.darkOnPrimaryContainer,
    },
  };
}

export const DEFAULT_THEME_ID = "material-you";

export const THEMES: ThemePreset[] = [
  defineTheme("material-you", "Material You", {
    primary: "oklch(0.55 0.18 155)",
    primaryForeground: "oklch(0.985 0 0)",
    ring: "oklch(0.55 0.18 155)",
    darkPrimary: "oklch(0.7 0.16 155)",
    darkPrimaryForeground: "oklch(0.15 0 0)",
    darkRing: "oklch(0.7 0.16 155)",
    darkPrimaryContainer: "oklch(0.42 0.11 155)",
    darkOnPrimaryContainer: "oklch(0.96 0 0)",
  }),
  defineTheme("azure", "Azure", {
    primary: "oklch(0.55 0.16 250)",
    primaryForeground: "oklch(0.985 0 0)",
    ring: "oklch(0.55 0.16 250)",
    darkPrimary: "oklch(0.72 0.15 250)",
    darkPrimaryForeground: "oklch(0.15 0 0)",
    darkRing: "oklch(0.72 0.15 250)",
    darkPrimaryContainer: "oklch(0.42 0.12 250)",
    darkOnPrimaryContainer: "oklch(0.96 0 0)",
  }),
  defineTheme("violet", "Violet", {
    primary: "oklch(0.55 0.2 295)",
    primaryForeground: "oklch(0.985 0 0)",
    ring: "oklch(0.55 0.2 295)",
    darkPrimary: "oklch(0.72 0.17 295)",
    darkPrimaryForeground: "oklch(0.15 0 0)",
    darkRing: "oklch(0.72 0.17 295)",
    darkPrimaryContainer: "oklch(0.42 0.14 295)",
    darkOnPrimaryContainer: "oklch(0.96 0 0)",
  }),
  defineTheme("amber", "Amber", {
    primary: "oklch(0.68 0.16 65)",
    primaryForeground: "oklch(0.18 0 0)",
    ring: "oklch(0.68 0.16 65)",
    darkPrimary: "oklch(0.78 0.15 70)",
    darkPrimaryForeground: "oklch(0.18 0 0)",
    darkRing: "oklch(0.78 0.15 70)",
    darkPrimaryContainer: "oklch(0.45 0.11 65)",
    darkOnPrimaryContainer: "oklch(0.97 0 0)",
  }),
];

export function getTheme(id: string): ThemePreset {
  return (
    THEMES.find((theme) => theme.id === id) ??
    THEMES.find((theme) => theme.id === DEFAULT_THEME_ID) ??
    THEMES[0]
  );
}
