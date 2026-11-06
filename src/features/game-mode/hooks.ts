import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { useBackendEvents } from "@/hooks/useBackendEvents";
import { sessionsKey } from "@/hooks/useMirrors";
import { events } from "@/lib/events";
import { DEFAULT_MIRROR_OPTIONS } from "@/lib/mirror";
import { errorMessage, ipc } from "@/lib/ipc";
import type { Controller, GameOptions } from "@/types";
import type { GameVideo } from "./store";

const controllersKey = ["game-controllers"] as const;
const audioOutputsKey = ["audio-outputs"] as const;

/** The running Game Mode session for `serial`, if any. */
export function useGameSession(serial: string | null) {
  const queryClient = useQueryClient();
  const refresh = () => queryClient.invalidateQueries({ queryKey: sessionsKey });

  useBackendEvents(
    (track) => {
      track(events.mirrorSessionStarted(refresh));
      track(events.mirrorSessionExited(refresh));
      track(events.gameUpdated(refresh));
      track(events.gameWarning(({ payload }) => toast.warning(payload.message)));
    },
    [queryClient],
  );

  const sessions = useQuery({ queryKey: sessionsKey, queryFn: () => ipc.mirror.listSessions() });
  return sessions.data?.find((session) => session.game && session.serial === serial) ?? null;
}

export function useStartGame() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      serial,
      video,
      game,
    }: {
      serial: string;
      video: GameVideo;
      game: GameOptions;
    }) =>
      ipc.mirror.openWindow({
        serial,
        kind: "mirror",
        package: null,
        label: "Game Mode",
        options: {
          ...DEFAULT_MIRROR_OPTIONS,
          resolution: video.resolution,
          fps: video.fps,
          bitrate: video.bitrate,
          stay_awake: video.stayAwake,
          // Game Mode plays the sound itself, with its own settings.
          audio: false,
          game,
        },
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: sessionsKey }),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

export function useStopGame() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => ipc.mirror.stopSession(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: sessionsKey }),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Controllers the computer sees, kept current while a session forwards them. */
export function useControllers() {
  const queryClient = useQueryClient();

  useBackendEvents(
    (track) => {
      track(
        events.gameControllers(({ payload }) =>
          queryClient.setQueryData<Controller[]>(controllersKey, payload),
        ),
      );
    },
    [queryClient],
  );

  return useQuery({ queryKey: controllersKey, queryFn: () => ipc.game.controllers() });
}

export function useAudioOutputs() {
  return useQuery({
    queryKey: audioOutputsKey,
    queryFn: () => ipc.game.audioOutputs(),
    staleTime: Infinity,
  });
}
