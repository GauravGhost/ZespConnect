import type { UnlistenFn } from "@tauri-apps/api/event";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";
import type { SoundRoute } from "@/types";

import { useBackendEvents } from "./useBackendEvents";

/** A per-device background link the backend runs and reports through an event. */
interface LinkSource {
  key: readonly string[];
  list: () => Promise<string[]>;
  set: (serial: string, on: boolean) => Promise<void>;
  onChange: (handler: () => void) => Promise<UnlistenFn>;
}

const CLIPBOARD_SYNC: LinkSource = {
  key: ["clipboard-sync"],
  list: ipc.clipboard.syncing,
  set: ipc.clipboard.setSync,
  onChange: (handler) => events.clipboardSyncChanged(handler),
};

const CONTROLLER_PASSTHROUGH: LinkSource = {
  key: ["controller-passthrough"],
  list: ipc.game.passthrough,
  set: ipc.game.setPassthrough,
  onChange: (handler) => events.controllerPassthroughChanged(handler),
};

function useDeviceLink(source: LinkSource, serial: string | null) {
  const queryClient = useQueryClient();
  const refresh = () => queryClient.invalidateQueries({ queryKey: source.key });

  // The link can also end on its own (cable out, phone rebooted), so follow the backend's events.
  useBackendEvents((track) => track(source.onChange(refresh)), [queryClient]);

  const { data } = useQuery({ queryKey: source.key, queryFn: source.list });
  const mutation = useMutation({
    mutationFn: (on: boolean) => source.set(serial as string, on),
    onError: (error) => toast.error(errorMessage(error)),
    onSettled: refresh,
  });

  return {
    on: serial != null && (data ?? []).includes(serial),
    pending: mutation.isPending,
    set: (on: boolean) => {
      if (serial) mutation.mutate(on);
    },
  };
}

const PHONE_AUDIO_KEY = ["phone-audio"] as const;

/** Where `serial`'s sound plays: the phone, the computer, or both. */
export function usePhoneAudio(serial: string | null) {
  const queryClient = useQueryClient();
  const refresh = () => queryClient.invalidateQueries({ queryKey: PHONE_AUDIO_KEY });

  useBackendEvents((track) => track(events.phoneAudioChanged(refresh)), [queryClient]);

  const { data } = useQuery({ queryKey: PHONE_AUDIO_KEY, queryFn: ipc.audio.routes });
  const mutation = useMutation({
    mutationFn: (route: SoundRoute) => ipc.audio.setRoute(serial as string, route),
    onError: (error) => toast.error(errorMessage(error)),
    onSettled: refresh,
  });

  const route: SoundRoute = (serial && data?.[serial]) || "phone";
  return {
    route,
    /** The sound plays on the computer, alone or with the phone. */
    on: route !== "phone",
    pending: mutation.isPending,
    set: (next: SoundRoute) => {
      if (serial) mutation.mutate(next);
    },
  };
}

/** Whether `serial`'s clipboard syncs with the computer's. */
export function useClipboardSync(serial: string | null) {
  return useDeviceLink(CLIPBOARD_SYNC, serial);
}

/** Whether `serial`'s mirror windows take this computer's controllers. */
export function useControllerPassthrough(serial: string | null) {
  return useDeviceLink(CONTROLLER_PASSTHROUGH, serial);
}
