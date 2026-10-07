import { useMutation, useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { errorMessage, ipc } from "@/lib/ipc";
import type { QuickState, QuickToggleKey, RebootMode } from "@/types";

export const quickStateKey = (serial: string) => ["quick-state", serial] as const;

/** Boolean fields of `QuickState`, i.e. the ones a toggle can flip. */
type ToggleField =
  | "wifi"
  | "bluetooth"
  | "dark_mode"
  | "airplane_mode"
  | "mobile_data"
  | "location"
  | "dnd"
  | "auto_rotate";

const TOGGLE_FIELD: Record<QuickToggleKey, ToggleField> = {
  wifi: "wifi",
  bluetooth: "bluetooth",
  darkmode: "dark_mode",
  airplane: "airplane_mode",
  data: "mobile_data",
  location: "location",
  dnd: "dnd",
  rotate: "auto_rotate",
};

/** Toggles, brightness and volume for the active device. */
export function useQuickState(serial: string | null) {
  return useQuery({
    queryKey: quickStateKey(serial ?? ""),
    queryFn: () => ipc.controls.quickState(serial as string),
    enabled: Boolean(serial),
    // Poll so changes made on the phone itself show up here.
    refetchInterval: 5_000,
  });
}

/** Re-read the true state a moment after a change settles (the device is slow to apply it). */
function refetchSoon(queryClient: QueryClient, serial: string) {
  window.setTimeout(() => {
    queryClient.invalidateQueries({ queryKey: quickStateKey(serial) });
  }, 1000);
}

/** Optimistically apply `patch`, keeping the previous value for rollback. */
async function optimisticPatch(
  queryClient: QueryClient,
  serial: string,
  patch: Partial<QuickState>,
) {
  await queryClient.cancelQueries({ queryKey: quickStateKey(serial) });
  const previous = queryClient.getQueryData<QuickState>(quickStateKey(serial));
  if (previous) {
    queryClient.setQueryData<QuickState>(quickStateKey(serial), { ...previous, ...patch });
  }
  return { previous };
}

function rollback(
  queryClient: QueryClient,
  serial: string,
  context: { previous?: QuickState } | undefined,
  error: unknown,
) {
  if (context?.previous) {
    queryClient.setQueryData(quickStateKey(serial), context.previous);
  }
  toast.error(errorMessage(error));
}

export function useSetToggle(serial: string | null) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ key, on }: { key: QuickToggleKey; on: boolean }) =>
      ipc.controls.setToggle(serial as string, key, on),
    onMutate: ({ key, on }) =>
      serial ? optimisticPatch(queryClient, serial, { [TOGGLE_FIELD[key]]: on }) : undefined,
    onError: (error, _vars, context) => {
      if (serial) rollback(queryClient, serial, context, error);
    },
    onSettled: () => {
      if (serial) refetchSoon(queryClient, serial);
    },
  });
}

export function useSetBrightness(serial: string | null) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (percent: number) => ipc.controls.setBrightness(serial as string, percent),
    onMutate: (percent) => {
      if (!serial) return undefined;
      const current = queryClient.getQueryData<QuickState>(quickStateKey(serial));
      const raw = current ? Math.round((percent / 100) * current.brightness_max) : percent;
      return optimisticPatch(queryClient, serial, { brightness: raw });
    },
    onError: (error, _percent, context) => {
      if (serial) rollback(queryClient, serial, context, error);
    },
    onSettled: () => {
      if (serial) refetchSoon(queryClient, serial);
    },
  });
}

export function useSetVolume(serial: string | null) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (percent: number) => ipc.controls.setVolume(serial as string, percent),
    onMutate: (percent) =>
      serial ? optimisticPatch(queryClient, serial, { volume: percent }) : undefined,
    onError: (error, _percent, context) => {
      if (serial) rollback(queryClient, serial, context, error);
    },
    onSettled: () => {
      if (serial) refetchSoon(queryClient, serial);
    },
  });
}

export function useReboot(serial: string | null) {
  return useMutation({
    mutationFn: (mode: RebootMode) => ipc.controls.reboot(serial as string, mode),
    onSuccess: () => toast.success("Reboot command sent"),
    onError: (error) => toast.error(errorMessage(error)),
  });
}
