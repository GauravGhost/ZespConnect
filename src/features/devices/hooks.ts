import { useMutation, useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { DEVICE_INFO_POLL_MS, deviceInfoKey } from "@/hooks/useDevices";
import { errorMessage, ipc } from "@/lib/ipc";
import type { DeviceInfo, QuickState, QuickToggleKey, RebootMode } from "@/types";

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

export const TOGGLE_FIELD: Record<QuickToggleKey, ToggleField> = {
  wifi: "wifi",
  bluetooth: "bluetooth",
  darkmode: "dark_mode",
  airplane: "airplane_mode",
  data: "mobile_data",
  location: "location",
  dnd: "dnd",
  rotate: "auto_rotate",
};

/** How long the device takes to apply a change before its state is worth re-reading. */
const SETTLE_DELAY_MS = 1000;

/** The quick settings part of the device info query, polled so changes made on the phone show up. */
export function useQuickState(serial: string | null) {
  return useQuery({
    queryKey: deviceInfoKey(serial ?? ""),
    queryFn: () => ipc.devices.info(serial as string),
    enabled: Boolean(serial),
    select: (info) => info.quick,
    refetchInterval: DEVICE_INFO_POLL_MS,
  });
}

function quickStateOf(queryClient: QueryClient, serial: string): QuickState | undefined {
  return queryClient.getQueryData<DeviceInfo>(deviceInfoKey(serial))?.quick;
}

function setQuickState(queryClient: QueryClient, serial: string, quick: QuickState) {
  queryClient.setQueryData<DeviceInfo>(deviceInfoKey(serial), (info) =>
    info ? { ...info, quick } : info,
  );
}

/** Re-read the true state a moment after a change settles (the device is slow to apply it). */
function refetchSoon(queryClient: QueryClient, serial: string) {
  window.setTimeout(() => {
    queryClient.invalidateQueries({ queryKey: deviceInfoKey(serial) });
  }, SETTLE_DELAY_MS);
}

/** Optimistically apply `patch`, keeping the previous value for rollback. */
async function optimisticPatch(
  queryClient: QueryClient,
  serial: string,
  patch: Partial<QuickState>,
) {
  await queryClient.cancelQueries({ queryKey: deviceInfoKey(serial) });
  const previous = quickStateOf(queryClient, serial);
  if (previous) setQuickState(queryClient, serial, { ...previous, ...patch });
  return { previous };
}

function rollback(
  queryClient: QueryClient,
  serial: string,
  context: { previous?: QuickState } | undefined,
  error: unknown,
) {
  if (context?.previous) {
    setQuickState(queryClient, serial, context.previous);
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
      const current = quickStateOf(queryClient, serial);
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
