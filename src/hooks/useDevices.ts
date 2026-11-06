import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

import { useAppStore } from "@/app/store";
import { events } from "@/lib/events";
import { ipc } from "@/lib/ipc";
import type { Device } from "@/types";

import { useBackendEvents } from "./useBackendEvents";

export const devicesKey = ["devices"] as const;
export const deviceInfoKey = (serial: string) => ["device-info", serial] as const;

/** Live device list: seeded by `list_devices`, kept fresh by `devices:changed` (no polling). */
export function useDevices() {
  const queryClient = useQueryClient();

  useBackendEvents(
    (track) => {
      track(
        events.devicesChanged((event) =>
          queryClient.setQueryData<Device[]>(devicesKey, event.payload),
        ),
      );
    },
    [queryClient],
  );

  return useQuery({
    queryKey: devicesKey,
    queryFn: () => ipc.devices.list(),
  });
}

/** The active device, kept consistent with the store as the device list changes. */
export function useActiveDevice() {
  const { data: devices = [], isLoading } = useDevices();
  const selectedSerial = useAppStore((s) => s.selectedSerial);
  const setSelectedSerial = useAppStore((s) => s.setSelectedSerial);

  useEffect(() => {
    if (devices.length === 0) {
      if (selectedSerial) setSelectedSerial(null);
      return;
    }
    const exists = devices.some((d) => d.serial === selectedSerial);
    if (!exists) {
      const preferred = devices.find((d) => d.connected) ?? devices[0];
      setSelectedSerial(preferred.serial);
    }
  }, [devices, selectedSerial, setSelectedSerial]);

  const device = devices.find((d) => d.serial === selectedSerial) ?? null;
  return { device, devices, selectedSerial, setSelectedSerial, isLoading };
}

/** Device info polling interval; one observer drives it so the phone runs one script per tick. */
export const DEVICE_INFO_POLL_MS = 5_000;

/**
 * Detailed info (model/android/battery/quick state) for the active device. Read-only here:
 * `useQuickState` polls the same query while the quick settings shade is shown.
 */
export function useDeviceInfo(serial: string | null) {
  return useQuery({
    queryKey: deviceInfoKey(serial ?? ""),
    queryFn: () => ipc.devices.info(serial as string),
    enabled: Boolean(serial),
  });
}
