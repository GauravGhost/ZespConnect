import type { UnlistenFn } from "@tauri-apps/api/event";
import { useMutation } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";
import type { MediaKey, NowPlaying } from "@/types";

/** Each watch call replaces the previous one, so they must reach the backend in order. */
let watchQueue: Promise<void> = Promise.resolve();

function setWatched(serial: string | null) {
  watchQueue = watchQueue
    .then(() => ipc.media.watch(serial))
    .catch((error: unknown) => {
      toast.error(errorMessage(error));
    });
}

/**
 * What `serial` is playing, watched while this component is mounted and the window is visible.
 * `nowPlaying` is `undefined` until the phone first answers and `null` when nothing plays.
 */
export function useNowPlaying(serial: string) {
  const [latest, setLatest] = useState<{ serial: string; nowPlaying: NowPlaying | null }>();

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    // A hidden window (closed to the tray) stops the watch so the phone is left alone.
    const watchWhileVisible = () => {
      if (unlisten) setWatched(document.visibilityState === "visible" ? serial : null);
    };

    events
      .nowPlayingChanged(({ payload }) => {
        if (payload.serial === serial) setLatest(payload);
      })
      .then((stop) => {
        if (disposed) {
          stop();
          return;
        }
        unlisten = stop;
        watchWhileVisible();
      })
      .catch(() => {
        /* backend not ready yet */
      });
    document.addEventListener("visibilitychange", watchWhileVisible);

    return () => {
      disposed = true;
      document.removeEventListener("visibilitychange", watchWhileVisible);
      if (unlisten) {
        unlisten();
        setWatched(null);
      }
    };
  }, [serial]);

  const press = useMutation({
    mutationFn: (key: MediaKey) => ipc.media.sendKey(serial, key),
    onError: (error) => toast.error(errorMessage(error)),
  });

  return {
    nowPlaying: latest?.serial === serial ? latest.nowPlaying : undefined,
    pressing: press.isPending,
    press: (key: MediaKey) => press.mutate(key),
  };
}
