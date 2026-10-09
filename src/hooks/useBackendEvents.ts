import type { UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, type DependencyList } from "react";

/** Subscribe to backend events for the component's life; register listeners via `track`. */
export function useBackendEvents(
  subscribe: (track: (pending: Promise<UnlistenFn>) => void) => void,
  deps: DependencyList,
) {
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];
    let disposed = false;

    const track = (pending: Promise<UnlistenFn>) => {
      pending
        .then((unlisten) => {
          if (disposed) unlisten();
          else unlisteners.push(unlisten);
        })
        .catch(() => {
          /* backend not ready yet */
        });
    };

    subscribe(track);
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
}
