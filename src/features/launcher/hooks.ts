import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";
import type { AppInfo, LauncherTarget } from "@/types";
import { rankLauncherApps } from "./rank";
import { readRecents, rememberRecent } from "./recents";

/** State and actions for the launcher popup. It reloads every time the backend shows it. */
export function useLauncher() {
  const [target, setTarget] = useState<LauncherTarget | null>(null);
  const [apps, setApps] = useState<AppInfo[]>([]);
  const [recents, setRecents] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [query, setQueryState] = useState("");
  const [highlightedPackage, setHighlightedPackage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [opening, setOpening] = useState(false);
  // Each show starts a new load; a slower earlier load must not overwrite it.
  const loadGeneration = useRef(0);
  const readySerial = useRef<string | null>(null);

  const load = useCallback(async () => {
    const generation = ++loadGeneration.current;
    const isCurrent = () => generation === loadGeneration.current;
    const next = await ipc.launcher.target();
    if (!isCurrent()) return;

    setQueryState("");
    setHighlightedPackage(null);
    setError(null);
    setApps([]);
    setTarget(next);
    readySerial.current = next.kind === "ready" ? next.serial : null;
    if (next.kind !== "ready") return;

    setRecents(readRecents(next.serial));
    setLoading(true);
    try {
      const list = await ipc.launcher.apps(next.serial);
      if (isCurrent()) setApps(list);
    } catch (cause) {
      if (isCurrent()) setError(errorMessage(cause));
    } finally {
      if (isCurrent()) setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
    const shown = events.launcherShown(() => void load());
    // The highlight is kept by package, so it stays on the same app when the list is replaced.
    const changed = events.launcherAppsChanged(({ payload }) => {
      if (payload.serial === readySerial.current) setApps(payload.apps);
    });
    return () => {
      void shown.then((unlisten) => unlisten());
      void changed.then((unlisten) => unlisten());
    };
  }, [load]);

  const ranked = useMemo(() => rankLauncherApps(apps, query, recents), [apps, query, recents]);
  const found = ranked.findIndex((app) => app.package === highlightedPackage);
  const highlight = found === -1 ? 0 : found;
  const selected: AppInfo | undefined = ranked[highlight];

  function setQuery(value: string) {
    setQueryState(value);
    setHighlightedPackage(null);
  }

  function move(delta: number) {
    if (ranked.length === 0) return;
    const next = Math.min(Math.max(highlight + delta, 0), ranked.length - 1);
    setHighlightedPackage(ranked[next].package);
  }

  async function open() {
    if (target?.kind !== "ready" || !selected || opening) return;
    setOpening(true);
    setError(null);
    try {
      await ipc.mirror.startAppWindow(target.serial, selected.package, selected.label);
      rememberRecent(target.serial, selected.package);
      await ipc.launcher.hide();
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setOpening(false);
    }
  }

  return { target, apps: ranked, query, setQuery, highlight, move, open, error, loading, opening };
}
