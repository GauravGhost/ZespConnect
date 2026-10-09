import type { UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, type DependencyList } from "react";

/**
 * Subscribe to backend events for the life of the component and dispose every
 * listener on unmount (or immediately if it resolves after unmount). Register
 * listeners through the `track` helper.
 */
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
