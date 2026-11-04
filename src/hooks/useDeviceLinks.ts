import type { UnlistenFn } from "@tauri-apps/api/event";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";

import { useBackendEvents } from "./useBackendEvents";

/** A per-device background link the backend runs and reports through an event. */
interface LinkSource {
  key: readonly string[];
  list: () => Promise<string[]>;
  set: (serial: string, on: boolean) => Promise<void>;
  onChange: (handler: () => void) => Promise<UnlistenFn>;
}

const PHONE_AUDIO: LinkSource = {
  key: ["phone-audio"],
  list: ipc.audio.onComputer,
  set: ipc.audio.setOnComputer,
  onChange: (handler) => events.phoneAudioChanged(handler),
};

const CLIPBOARD_SYNC: LinkSource = {
  key: ["clipboard-sync"],
  list: ipc.clipboard.syncing,
  set: ipc.clipboard.setSync,
  onChange: (handler) => events.clipboardSyncChanged(handler),
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

/** Whether `serial`'s sound plays on the computer instead of the phone. */
export function usePhoneAudio(serial: string | null) {
  return useDeviceLink(PHONE_AUDIO, serial);
}

/** Whether `serial`'s clipboard syncs with the computer's. */
export function useClipboardSync(serial: string | null) {
  return useDeviceLink(CLIPBOARD_SYNC, serial);
}
