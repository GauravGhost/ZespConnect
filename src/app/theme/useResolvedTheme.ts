import { useEffect, useState } from "react";

import { useAppStore } from "@/app/store";

function useSystemDark(): boolean {
  const [dark, setDark] = useState(() => window.matchMedia("(prefers-color-scheme: dark)").matches);

  useEffect(() => {
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = (event: MediaQueryListEvent) => setDark(event.matches);
    mql.addEventListener("change", onChange);
    return () => mql.removeEventListener("change", onChange);
  }, []);

  return dark;
}

/** The concrete mode in effect right now ("system" resolved via the OS). */
export function useResolvedTheme(): "light" | "dark" {
  const themeMode = useAppStore((state) => state.themeMode);
  const systemDark = useSystemDark();
  return themeMode === "system" ? (systemDark ? "dark" : "light") : themeMode;
}
