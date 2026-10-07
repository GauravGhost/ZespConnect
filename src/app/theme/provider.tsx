import { useEffect } from "react";

import { useAppStore } from "@/app/store";
import { getTheme } from "./presets";
import { useResolvedTheme } from "./useResolvedTheme";

/**
 * Applies the active theme by writing its tokens as CSS custom properties on
 * `<html>` and toggling the `.dark` class (which powers the `dark:` variants).
 * Palettes come from `presets.ts`; this component owns no colors itself.
 */
export function ThemeProvider() {
  const themeId = useAppStore((state) => state.themeId);
  const resolved = useResolvedTheme();

  useEffect(() => {
    const root = document.documentElement;
    const tokens = getTheme(themeId)[resolved];

    for (const [name, value] of Object.entries(tokens)) {
      root.style.setProperty(name, value);
    }

    root.classList.toggle("dark", resolved === "dark");
    root.dataset.theme = themeId;
    root.style.colorScheme = resolved;
  }, [themeId, resolved]);

  return null;
}
